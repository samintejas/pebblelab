//! Where this page runs. One build serves the pebblelab account and every app; the server says which one this
//! address is (`account`, `fin`, or `all` when one address serves everything) and where the others live.
//!
//! Signing in happens in the account. An app that has no session sends the browser there with `?app=<id>`;
//! once signed in, the account asks the server for a one-time code and sends the browser to the app's
//! `/auth/callback`, which swaps it for a session of the app's own.

use leptos::prelude::*;
use pebblelab_api::*;

use crate::api;

/// The site, once the server has said. Until then, and if it cannot, everything is at this address.
#[derive(Clone, Copy)]
pub struct SiteCtx(pub RwSignal<Option<Site>>);

pub fn provide() -> SiteCtx {
    let ctx = SiteCtx(RwSignal::new(None));
    provide_context(ctx);
    leptos::task::spawn_local(async move {
        let site = api::get::<Site>("/site").await.unwrap_or_else(|_| fallback());
        ctx.0.set(Some(site));
    });
    ctx
}

pub fn use_site() -> SiteCtx {
    expect_context::<SiteCtx>()
}

fn origin() -> String {
    web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default()
}

/// One address serving everything: what a page that cannot reach the server assumes.
fn fallback() -> Site {
    let here = origin();
    Site {
        here: "all".into(),
        account_url: here.clone(),
        apps: APPS.iter().map(|a| SiteApp { id: a.id.into(), name: a.name.into(), summary: a.summary.into(), icon: a.icon.into(), url: here.clone() }).collect(),
    }
}

impl SiteCtx {
    pub fn ready(&self) -> bool {
        self.0.with(|s| s.is_some())
    }

    pub fn get(&self) -> Site {
        self.0.get().unwrap_or_else(fallback)
    }

    /// `account`, `fin` or `all`.
    pub fn here(&self) -> String {
        self.0.with(|s| s.as_ref().map(|s| s.here.clone())).unwrap_or_else(|| "all".into())
    }

    /// This address serves the account (alone, or with everything else).
    pub fn is_account(&self) -> bool {
        matches!(self.here().as_str(), "account" | "all")
    }

    /// This address serves fin and not the account: signing in happens elsewhere.
    pub fn is_fin_only(&self) -> bool {
        self.here() == "fin"
    }

    /// A link into the account: `account_href("/account/family")`. Relative when the account is here.
    pub fn account_href(&self, path: &str) -> String {
        if self.is_account() { path.to_string() } else { format!("{}{path}", self.get().account_url) }
    }

    /// A link into an app. Relative when that app is here.
    pub fn app_href(&self, app: &str, path: &str) -> String {
        let site = self.get();
        match site.url(app) {
            Some(u) if u != origin() && !site.single() => format!("{u}{path}"),
            _ => path.to_string(),
        }
    }
}

/// Leave this address for another one (a full page load, not a route change).
pub fn go_external(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(url);
    }
}

/// `?app=fin` in the address: the app that sent the browser here to sign in.
pub fn requested_app() -> Option<String> {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let q: std::collections::HashMap<String, String> = serde_urlencoded::from_str(search.trim_start_matches('?')).unwrap_or_default();
    q.get("app").filter(|a| app(a).is_some_and(|a| a.id != ACCOUNT.id)).cloned()
}

const PENDING: &str = "pebblelab:continue";

/// Remember which app asked for a sign-in across a trip to Google or GitHub.
pub fn remember_app(app: Option<&str>) {
    if let Some(s) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        match app {
            Some(a) => {
                let _ = s.set_item(PENDING, a);
            }
            None => {
                let _ = s.remove_item(PENDING);
            }
        }
    }
}

fn take_remembered() -> Option<String> {
    let s = web_sys::window().and_then(|w| w.session_storage().ok().flatten())?;
    let a = s.get_item(PENDING).ok().flatten();
    let _ = s.remove_item(PENDING);
    a
}

/// Signed in to the account: go where the person was headed. An app at another address gets this browser
/// handed to it; an app at this address just opens; with no app asked for, the account opens. Returns the
/// path to route to when the destination is here.
pub async fn continue_after_sign_in(site: &SiteCtx, app: Option<String>) -> Result<String, api::ApiError> {
    let app = app.or_else(take_remembered);
    let s = site.get();
    match app.as_deref() {
        Some(a) if !s.single() && s.url(a).is_some_and(|u| u != origin()) => {
            let h = api::post::<Handoff>("/auth/handoff", &HandoffRequest { app: a.to_string() }).await?;
            go_external(&h.url);
            Ok(String::new())
        }
        Some("fin") => Ok("/transactions".into()),
        // one address serving everything: the person most likely came for fin
        _ if s.single() => Ok("/transactions".into()),
        _ => Ok("/account".into()),
    }
}

/// An app with no session: send the browser to the account to sign in, and back.
pub fn sign_in_elsewhere(site: &SiteCtx, app: &str) {
    go_external(&format!("{}/signin?app={app}", site.get().account_url));
}
