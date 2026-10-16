//! `/connect`: an MCP client (Claude, ChatGPT, ...) asks to use your account. The server's `/authorize` sends the
//! browser here with the OAuth request; you sign in if needed, see who is asking and where approving sends you,
//! choose what it may do, and go back to the client with a code (or a no).

use dots_ui::prelude::*;
use leptos::prelude::*;
use pebblelab_api::*;

use super::settings::PERMS;
use crate::api;

/// The connect request waiting for a sign-in to finish.
pub const CONNECT_KEY: &str = "pebblelab:connect";

/// The page to come back to after signing in, if a connect request is waiting. Taken once.
pub fn take_connect() -> Option<String> {
    let s = web_sys::window()?.session_storage().ok().flatten()?;
    let v = s.get_item(CONNECT_KEY).ok().flatten();
    let _ = s.remove_item(CONNECT_KEY);
    v.filter(|p| p.starts_with("/connect?"))
}

fn request() -> ConnectRequest {
    let q = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    serde_urlencoded::from_str(q.trim_start_matches('?')).unwrap_or_default()
}

/// Sign in first, then come back here.
fn sign_in_first() {
    let Some(w) = web_sys::window() else { return };
    let here = format!("/connect{}", w.location().search().unwrap_or_default());
    if let Some(s) = w.session_storage().ok().flatten() {
        let _ = s.set_item(CONNECT_KEY, &here);
    }
    api::clear_token();
    // a full load: this can run outside the page's reactive scope, after a request
    let _ = w.location().set_href(if api::is_hub() { "/signin?to=connect" } else { "/signin" });
}

#[component]
pub fn Connect() -> impl IntoView {
    let req = request();
    let info = RwSignal::new(None::<ConnectInfo>);
    let err = RwSignal::new(None::<String>);
    let scopes = RwSignal::new(Vec::<String>::new());
    let busy = RwSignal::new(false);
    if api::load_token().is_none() {
        sign_in_first();
    } else {
        let r = req.clone();
        leptos::task::spawn_local(async move {
            match api::post::<ConnectInfo>("/connect/info", &r).await {
                Ok(i) => {
                    scopes.set(i.scopes.clone());
                    info.set(Some(i));
                }
                Err(e) => err.set(Some(e.message)),
            }
        });
    }
    let decide = move |allow: bool| {
        let d = ConnectDecision { request: req.clone(), allow, scopes: scopes.get_untracked() };
        busy.set(true);
        leptos::task::spawn_local(async move {
            match api::post::<ConnectRedirect>("/connect/decide", &d).await {
                Ok(r) => {
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().set_href(&r.redirect);
                    }
                }
                Err(e) if e.unauthorized() => sign_in_first(),
                Err(e) => {
                    err.set(Some(e.message));
                    busy.set(false);
                }
            }
        });
    };
    let (allow, deny) = (decide.clone(), decide);
    view! {
        <SkipLink/>
        <main id="main" tabindex="-1" class="d-status-page" data-kind="loading" style="min-height:100vh">
            {move || match (err.get(), info.get()) {
                (Some(m), None) => view! {
                    <div class="d-status-page__inner">
                        <h1 class="d-status-page__title">"this app cannot connect"</h1>
                        <p class="d-status-page__text"><span class="d-error" role="alert">{format!("error: {m}")}</span></p>
                        <div class="d-status-page__actions"><ButtonLink href="/" variant=ButtonVariant::Secondary>"go home"</ButtonLink></div>
                    </div>
                }.into_any(),
                (_, None) => view! {
                    <div class="d-status-page__inner"><Pebbles label="loading" large=true/></div>
                }.into_any(),
                (_, Some(i)) => {
                    let (allow, deny) = (allow.clone(), deny.clone());
                    view! {
                        <div class="d-card" style="width:100%;max-width:440px;text-align:left">
                            <div class="d-card__body" style="padding:24px;display:flex;flex-direction:column;gap:16px">
                                <div style="display:flex;flex-direction:column;gap:4px">
                                    <h1 class="heading-xl" style="margin:0;color:var(--text-strong)">{format!("connect {}", i.client_name)}</h1>
                                    <p style="margin:0;color:var(--text-secondary)">
                                        {format!("{} wants to use your pebblelab account. approving sends you back to ", i.client_name)}
                                        <b style="color:var(--text-strong);font-weight:500">{i.sends_to.clone()}</b>
                                        ". only allow it if that is where you started."
                                    </p>
                                </div>
                                <div role="group" aria-label="what it may do">
                                    {PERMS.map(|(label, scope)| view! {
                                        <label class="d-setrow"><span class="d-setrow__text"><b>{label}</b></span>
                                            <input type="checkbox" role="switch" class="d-switch" prop:checked=move || scopes.with(|l| l.iter().any(|s| s == scope))
                                                on:change=move |e| {
                                                    let on = event_target_checked(&e);
                                                    scopes.update(|l| { l.retain(|s| s != scope); if on { l.push(scope.into()); } });
                                                }/>
                                        </label>
                                    })}
                                </div>
                                <span class="d-hint">"it shows on your connectors page, where you can change this or cut it off."</span>
                                {move || err.get().map(|m| view! { <span class="d-error" role="alert">{format!("error: {m}")}</span> })}
                            </div>
                            <footer class="d-card__foot">
                                <button type="button" class="d-btn" disabled=move || busy.get() on:click=move |_| deny(false)>"cancel"</button>
                                <button type="button" class="d-btn d-btn--primary" disabled=move || busy.get() || scopes.with(|s| s.is_empty()) on:click=move |_| allow(true)>"allow"</button>
                            </footer>
                        </div>
                    }.into_any()
                }
            }}
            <div class="d-status-page__foot">"pebblelab"</div>
        </main>
    }
}
