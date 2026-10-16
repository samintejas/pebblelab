//! MCP clients connecting with OAuth (the MCP authorization spec): a client registers itself, the person approves
//! it on the hub, and the client swaps the approval for a connector token. Clients are public: no secret, PKCE
//! (S256) instead. The token is an ordinary connector, listed and revocable on the connectors page.

use sqlx::{AssertSqlSafe, Row};

use crate::api::*;
use crate::social::{hash, random};
use crate::{Caller, Error, Result, Store, clean};

/// What a client gets when it does not say: the same as a connector made on the page.
const DEFAULT_SCOPES: [&str; 2] = ["read", "transactions"];

/// An approval is swapped for a token within this long, or not at all.
const CODE_MINUTES: i64 = 10;

/// Schemes a redirect can never use: they would run or read something in the browser.
const BAD_SCHEMES: [&str; 7] = ["javascript", "data", "file", "vbscript", "blob", "about", "ftp"];

fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "[::1]"
}

/// The scheme and host of a url, or `None` when it is not one.
fn parts(uri: &str) -> Option<(String, String)> {
    let (scheme, rest) = uri.split_once(':')?;
    let ok = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
    if !ok {
        return None;
    }
    let host = rest.strip_prefix("//").map(|r| {
        let authority = r.split(['/', '?']).next().unwrap_or("");
        let host_port = authority.rsplit('@').next().unwrap_or("");
        if host_port.starts_with('[') {
            host_port.split_inclusive(']').next().unwrap_or("").to_string()
        } else {
            host_port.split(':').next().unwrap_or("").to_string()
        }
    });
    Some((scheme.to_lowercase(), host.unwrap_or_default().to_lowercase()))
}

/// A redirect a client may register: https, http back to this computer, or an app's own scheme. Never a fragment.
fn check_redirect(uri: &str) -> Result<()> {
    let bad = || Error::bad(format!("redirect_uri '{uri}' is not allowed: use https, http://localhost or an app's own scheme"));
    if uri.contains('#') || uri.len() > 2000 || uri.chars().any(char::is_whitespace) {
        return Err(bad());
    }
    let (scheme, host) = parts(uri).ok_or_else(bad)?;
    match scheme.as_str() {
        "https" if !host.is_empty() => Ok(()),
        "http" if is_loopback(&host) => Ok(()),
        "http" | "https" => Err(bad()),
        s if BAD_SCHEMES.contains(&s) => Err(bad()),
        _ => Ok(()),
    }
}

/// Where a redirect goes, said for the person approving it.
fn sends_to(uri: &str) -> String {
    match parts(uri) {
        Some((s, h)) if s == "http" && is_loopback(&h) => "an app on this computer".into(),
        Some((s, h)) if s == "https" => h,
        Some((s, _)) => format!("the {s} app"),
        None => uri.to_string(),
    }
}

/// The scopes in an OAuth `scope` parameter that pebblelab knows. Anything else (`openid`, ...) is ignored.
fn wanted(scope: Option<&str>) -> Vec<String> {
    let asked: Vec<String> = scope.unwrap_or("").split_whitespace().filter(|s| SCOPES.contains(s)).map(String::from).collect();
    if asked.is_empty() { DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect() } else { SCOPES.iter().filter(|s| asked.iter().any(|a| a == *s)).map(|s| s.to_string()).collect() }
}

/// An approved request, swapped for a token.
pub struct Issued {
    pub token: String,
    pub scopes: Vec<String>,
}

impl Store {
    /// Dynamic client registration (RFC 7591): anyone may register; nothing is trusted until a person approves it.
    /// Returns the client id.
    pub async fn register_client(&self, name: &str, redirect_uris: &[String]) -> Result<String> {
        let name: String = clean(name).chars().take(80).collect();
        let name = if name.is_empty() { "an mcp client".to_string() } else { name };
        if redirect_uris.is_empty() || redirect_uris.len() > 10 {
            return Err(Error::bad("give between one and ten redirect_uris"));
        }
        for u in redirect_uris {
            check_redirect(u)?;
        }
        let id = random("plc_", 16);
        sqlx::query("INSERT INTO oauth_clients (id, name, redirect_uris) VALUES ($1, $2, $3)")
            .bind(&id)
            .bind(&name)
            .bind(redirect_uris.join("\n"))
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    /// Check a request against its client. An error here must not be sent to the redirect: it may not be the client's.
    async fn client_for(&self, r: &ConnectRequest) -> Result<String> {
        let row = sqlx::query("SELECT name, redirect_uris FROM oauth_clients WHERE id = $1")
            .bind(&r.client_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| Error::bad("this app is not registered here: connect it again from the app"))?;
        let uris: String = row.get("redirect_uris");
        if !uris.lines().any(|u| u == r.redirect_uri) {
            return Err(Error::bad("this app asked to be sent somewhere it did not register"));
        }
        Ok(row.get("name"))
    }

    /// What the consent page shows, once the request is known to be sound. Errors are for the page, not the client.
    pub async fn connect_info(&self, r: &ConnectRequest) -> Result<ConnectInfo> {
        let client_name = self.client_for(r).await?;
        if r.response_type != "code" {
            return Err(Error::bad("this app asked for something other than a sign-in code"));
        }
        if r.code_challenge_method != "S256" || r.code_challenge.len() < 43 || r.code_challenge.len() > 128 {
            return Err(Error::bad("this app did not send a PKCE (S256) challenge"));
        }
        Ok(ConnectInfo { client_name, sends_to: sends_to(&r.redirect_uri), scopes: wanted(r.scope.as_deref()) })
    }

    /// The person approved: a one-time code for the client. Only a person signed in may approve, never a token.
    pub async fn approve_connect(&self, c: &Caller, r: &ConnectRequest, scopes: &[String]) -> Result<String> {
        if !c.is_interactive() {
            return Err(Error::Forbidden("only you, signed in, can connect an app".into()));
        }
        self.connect_info(r).await?;
        if scopes.is_empty() {
            return Err(Error::bad("choose at least one permission"));
        }
        if let Some(bad) = scopes.iter().find(|s| !SCOPES.contains(&s.as_str())) {
            return Err(Error::bad(format!("unknown scope '{bad}', use one of {SCOPES:?}")));
        }
        let code = random("plo_", 32);
        sqlx::query(AssertSqlSafe(format!("DELETE FROM oauth_codes WHERE created_at < utc_text(now() - interval '{CODE_MINUTES} minutes')"))).execute(&self.pool).await?;
        sqlx::query("INSERT INTO oauth_codes (code_hash, client_id, user_id, redirect_uri, challenge, scopes) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(hash(&code))
            .bind(&r.client_id)
            .bind(c.user_id)
            .bind(&r.redirect_uri)
            .bind(&r.code_challenge)
            .bind(SCOPES.iter().filter(|s| scopes.iter().any(|a| a == *s)).copied().collect::<Vec<_>>().join(","))
            .execute(&self.pool)
            .await?;
        Ok(code)
    }

    /// The client swaps a code for a connector token. The code works once, for the client and redirect it was made
    /// for, and only with the verifier that matches its challenge (`verify` gets the stored challenge). A client
    /// connected again replaces its old token.
    pub async fn exchange_code(&self, code: &str, client_id: &str, redirect_uri: &str, verify: impl Fn(&str) -> bool) -> Result<Issued> {
        let refused = || Error::bad("the code is wrong, expired or already used");
        let row = sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM oauth_codes WHERE code_hash = $1 AND created_at > utc_text(now() - interval '{CODE_MINUTES} minutes') \
             RETURNING client_id, user_id, redirect_uri, challenge, scopes"
        )))
        .bind(hash(code.trim()))
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(refused)?;
        if row.get::<String, _>("client_id") != client_id || row.get::<String, _>("redirect_uri") != redirect_uri || !verify(&row.get::<String, _>("challenge")) {
            return Err(refused());
        }
        let (user_id, scopes): (i64, String) = (row.get("user_id"), row.get("scopes"));
        let name: String = sqlx::query_scalar("SELECT name FROM oauth_clients WHERE id = $1").bind(client_id).fetch_one(&self.pool).await?;
        let token = random("trc_", 32);
        let mut db = self.pool.begin().await?;
        sqlx::query("DELETE FROM tokens WHERE user_id = $1 AND kind = 'connector' AND client_id = $2").bind(user_id).bind(client_id).execute(&mut *db).await?;
        sqlx::query("INSERT INTO tokens (user_id, kind, name, token_hash, tail, scopes, client_id) VALUES ($1, 'connector', $2, $3, $4, $5, $6)")
            .bind(user_id)
            .bind(&name)
            .bind(hash(&token))
            .bind(&token[token.len() - 4..])
            .bind(&scopes)
            .bind(client_id)
            .execute(&mut *db)
            .await?;
        db.commit().await?;
        Ok(Issued { token, scopes: scopes.split(',').filter(|s| !s.is_empty()).map(String::from).collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirects_are_https_loopback_or_an_app() {
        for ok in ["https://claude.ai/api/mcp/auth_callback", "http://localhost:6274/oauth/callback", "http://127.0.0.1:33418/", "http://[::1]:80/cb", "cursor://anysphere.cursor-retrieval/oauth/callback"] {
            assert!(check_redirect(ok).is_ok(), "{ok}");
        }
        for bad in ["http://evil.example/cb", "javascript:alert(1)", "https://claude.ai/cb#x", "data:text/html,hi", "https:///nohost", "not a url", "http://localhost.evil.example/cb"] {
            assert!(check_redirect(bad).is_err(), "{bad}");
        }
        assert_eq!(sends_to("https://chatgpt.com/connector_platform_oauth_redirect"), "chatgpt.com");
        assert_eq!(sends_to("http://localhost:6274/cb"), "an app on this computer");
        assert_eq!(sends_to("cursor://x/cb"), "the cursor app");
    }

    #[test]
    fn unknown_scopes_are_ignored_and_none_means_the_default() {
        assert_eq!(wanted(None), ["read", "transactions"]);
        assert_eq!(wanted(Some("openid profile")), ["read", "transactions"]);
        assert_eq!(wanted(Some("edit read offline_access")), ["read", "edit"]);
    }
}
