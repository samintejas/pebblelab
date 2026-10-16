//! OAuth for MCP clients (the MCP authorization spec): how Claude, ChatGPT or any other client finds out where to
//! sign in, registers itself, sends the person to approve it, and swaps the approval for a connector token.
//!
//! The hub (or, without one, this server's public address) is the authorization server; `/mcp` on every address
//! is the protected resource. The person approves on the web app's `/connect` page; the rules are in
//! `pebblelab-core`.

use std::sync::Arc;

use axum::extract::{Extension, Form, RawQuery, State};
use axum::http::header::{CACHE_CONTROL, HOST, LOCATION, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use pebblelab_core::api::*;
use pebblelab_core::{Error, Store};

use crate::limit::Limits;
use crate::oauth::Oauth;
use crate::rest::{ApiError, Auth, ClientIp};

/// The address this request came in at, when it is one of ours (the hub, fin, the public address). In development,
/// with none of them set, whatever the Host header says.
pub fn origin(o: &Oauth, headers: &HeaderMap) -> String {
    let host = headers.get(HOST).and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let ours = [o.hub(), o.fin(), Some(o.public_url())].into_iter().flatten().filter(|u| !u.is_empty());
    for u in ours.clone() {
        if u.split_once("://").map(|(_, h)| h.split('/').next().unwrap_or("")) == Some(host.as_str()) {
            return u.to_string();
        }
    }
    match ours.last() {
        Some(u) => u.to_string(),
        None if host.is_empty() => "http://localhost".into(),
        None => format!("http://{host}"),
    }
}

/// The authorization server: the hub, where people sign in.
fn issuer(o: &Oauth, headers: &HeaderMap) -> String {
    o.hub().map(str::to_string).unwrap_or_else(|| if o.public_url().is_empty() { origin(o, headers) } else { o.public_url().to_string() })
}

/// What `/mcp` answers without a token: 401, and where to read how to get one.
pub fn unauthorized(o: &Oauth, headers: &HeaderMap) -> Response {
    let meta = format!("Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\"", origin(o, headers));
    let mut r = (StatusCode::UNAUTHORIZED, Json(json!({ "error": "sign in: connect this app from pebblelab, or send a connector token" }))).into_response();
    if let Ok(v) = HeaderValue::from_str(&meta) {
        r.headers_mut().insert(WWW_AUTHENTICATE, v);
    }
    r
}

/// RFC 9728: the resource, and who issues tokens for it.
pub async fn protected_resource(Extension(o): Extension<Arc<Oauth>>, headers: HeaderMap) -> Json<Value> {
    Json(json!({
        "resource": format!("{}/mcp", origin(&o, &headers)),
        "authorization_servers": [issuer(&o, &headers)],
        "scopes_supported": SCOPES,
        "bearer_methods_supported": ["header"],
        "resource_name": "pebblelab",
    }))
}

/// RFC 8414: where to register, send the person, and swap the code.
pub async fn authorization_server(Extension(o): Extension<Arc<Oauth>>, headers: HeaderMap) -> Json<Value> {
    let i = issuer(&o, &headers);
    Json(json!({
        "issuer": i,
        "authorization_endpoint": format!("{i}/authorize"),
        "token_endpoint": format!("{i}/token"),
        "registration_endpoint": format!("{i}/register"),
        "scopes_supported": SCOPES,
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "authorization_response_iss_parameter_supported": true,
    }))
}

/// An OAuth error, as RFC 6749 shapes it.
fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    let mut r = (status, Json(json!({ "error": error, "error_description": description }))).into_response();
    r.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

#[derive(Deserialize)]
pub struct Registration {
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    client_name: String,
}

/// RFC 7591 dynamic client registration. Public clients only: whatever auth method is asked for, it is `none`.
pub async fn register(State(s): State<Store>, Extension(l): Extension<Arc<Limits>>, ClientIp(ip): ClientIp, Json(b): Json<Registration>) -> Response {
    if let Err(e) = l.register.hit(&ip) {
        return oauth_error(StatusCode::TOO_MANY_REQUESTS, "invalid_request", &e.message());
    }
    match s.register_client(&b.client_name, &b.redirect_uris).await {
        Ok(id) => {
            let now = chrono::Utc::now().timestamp();
            (StatusCode::CREATED, Json(json!({
                "client_id": id,
                "client_id_issued_at": now,
                "client_name": b.client_name,
                "redirect_uris": b.redirect_uris,
                "token_endpoint_auth_method": "none",
                "grant_types": ["authorization_code"],
                "response_types": ["code"],
            })))
                .into_response()
        }
        Err(Error::BadRequest(m)) => oauth_error(StatusCode::BAD_REQUEST, "invalid_redirect_uri", &m),
        Err(e) => ApiError::from(e).into_response(),
    }
}

/// The browser arrives from the client: show the consent page, which checks the request and asks the person.
pub async fn authorize(RawQuery(q): RawQuery) -> Response {
    let to = format!("/connect?{}", q.unwrap_or_default());
    let mut r = (StatusCode::SEE_OTHER, [(LOCATION, to)]).into_response();
    r.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

/// The consent page asks what it is approving. Errors are for the page to show: never send them to the client.
pub async fn info(State(s): State<Store>, Json(r): Json<ConnectRequest>) -> Result<Json<ConnectInfo>, ApiError> {
    Ok(Json(s.connect_info(&r).await?))
}

/// The person's answer: where the browser goes next, back to the client with a code or `access_denied`.
pub async fn decide(State(s): State<Store>, Extension(o): Extension<Arc<Oauth>>, headers: HeaderMap, Auth(c, _): Auth, Json(d): Json<ConnectDecision>) -> Result<Json<ConnectRedirect>, ApiError> {
    let r = &d.request;
    let mut back = vec![];
    if d.allow {
        back.push(("code", s.approve_connect(&c, r, &d.scopes).await?));
    } else {
        // only send someone back to an address the client registered
        s.connect_info(r).await?;
        back.push(("error", "access_denied".to_string()));
    }
    if let Some(state) = &r.state {
        back.push(("state", state.clone()));
    }
    back.push(("iss", issuer(&o, &headers)));
    let sep = if r.redirect_uri.contains('?') { '&' } else { '?' };
    Ok(Json(ConnectRedirect { redirect: format!("{}{sep}{}", r.redirect_uri, serde_urlencoded::to_string(&back).unwrap_or_default()) }))
}

#[derive(Deserialize)]
pub struct TokenRequest {
    #[serde(default)]
    grant_type: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    redirect_uri: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    code_verifier: String,
}

/// The client swaps the code (and its PKCE verifier) for a connector token. It does not expire: the person
/// revokes it on the connectors page.
pub async fn token(State(s): State<Store>, Extension(l): Extension<Arc<Limits>>, ClientIp(ip): ClientIp, Form(t): Form<TokenRequest>) -> Response {
    if let Err(e) = l.sign_in_ip.hit(&ip) {
        return oauth_error(StatusCode::TOO_MANY_REQUESTS, "invalid_request", &e.message());
    }
    if t.grant_type != "authorization_code" {
        return oauth_error(StatusCode::BAD_REQUEST, "unsupported_grant_type", "only authorization_code: the token does not expire, so there is nothing to refresh");
    }
    if t.code.is_empty() || t.client_id.is_empty() || t.redirect_uri.is_empty() || !(43..=128).contains(&t.code_verifier.len()) {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "send code, client_id, redirect_uri and code_verifier");
    }
    let computed = URL_SAFE_NO_PAD.encode(Sha256::digest(t.code_verifier.as_bytes()));
    match s.exchange_code(&t.code, &t.client_id, &t.redirect_uri, |challenge| challenge == computed).await {
        Ok(i) => {
            let mut r = Json(json!({ "access_token": i.token, "token_type": "Bearer", "scope": i.scopes.join(" ") })).into_response();
            r.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            r
        }
        Err(Error::BadRequest(m)) => oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", &m),
        Err(e) => ApiError::from(e).into_response(),
    }
}
