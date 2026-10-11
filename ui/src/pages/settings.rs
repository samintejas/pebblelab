//! fin's own settings: how it looks and what it tells you about, and your data. Who you are, your family and
//! your tokens are the pebblelab account's.

use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use pebblelab_api::*;

use crate::api;
use crate::icons::*;
use crate::site::{self, use_site};
use crate::state::AppState;

/// Sections that moved to the account when fin became one app under it.
const MOVED: [&str; 4] = ["profile", "security", "family", "connectors"];

/// Save fin's preferences and refresh what the rest of the app knows.
fn patch_me(app: AppState, body: serde_json::Value) {
    leptos::task::spawn_local(async move {
        match api::patch::<User>("/me", &body).await {
            Ok(u) => app.me.update(|m| {
                if let Some(m) = m {
                    m.user = u;
                }
            }),
            Err(e) if e.status == 400 || e.status == 409 => app.error(e.message),
            Err(e) => app.fail(&e),
        }
    });
}

#[component]
pub fn Settings() -> impl IntoView {
    let app = expect_context::<AppState>();
    let site = use_site();
    // an old link to a section the account has now: follow it there
    let params = use_params_map();
    Effect::new(move |_| {
        let t = params.with(|p| p.get("tab")).unwrap_or_default();
        if site.ready() && MOVED.contains(&t.as_str()) {
            let href = site.account_href(&format!("/account/{t}"));
            if href.starts_with("http") {
                site::go_external(&href);
            } else if let Some(w) = web_sys::window() {
                let _ = w.location().set_href(&href);
            }
        }
    });
    let theme = use_theme();
    let theme_name = RwSignal::new(theme.get_untracked().as_str().to_string());
    Effect::new(move |_| {
        let t = Theme::parse(&theme_name.get()).unwrap_or_default();
        if theme.get_untracked() != t {
            theme.set(t);
        }
    });
    Effect::new(move |_| theme_name.set(theme.get().as_str().to_string()));
    let currency = RwSignal::new(app.currency());
    Effect::new(move |_| currency.set(app.currency()));
    let user = move || app.me.get().map(|m| m.user);
    let switch = move |label: &'static str, hint: &'static str, key: &'static str, get: fn(&User) -> bool| view! {
        <SettingRow label=label hint=hint toggle=true>
            <input type="checkbox" role="switch" class="d-switch" prop:checked=move || user().is_some_and(|u| get(&u))
                on:change=move |e| patch_me(app, serde_json::json!({ key: event_target_checked(&e) }))/>
        </SettingRow>
    };
    let export = move |_| {
        leptos::task::spawn_local(async move {
            if let Err(e) = api::download("/export.csv", "pebblelab-fin.csv").await {
                app.fail(&e);
            }
        });
    };
    let account = move |section: &str| site.account_href(&format!("/account/{section}"));
    view! {
        <div style="display:flex;flex-direction:column;gap:16px;max-width:1100px">
            <div style="display:flex;flex-direction:column;gap:4px">
                <h1 class="heading-xl">"settings"</h1>
                <span style="color:var(--text-secondary)">"how fin looks, what it tells you about, and your data."</span>
            </div>
            <SettingGrid>
                <section class="d-card" aria-labelledby="pr-t">
                    <header class="d-card__head"><h2 class="d-card__title" id="pr-t">"display"</h2></header>
                    <div class="d-card__body">
                        <SettingRows>
                            <SettingRow label="currency" r#for="pr-cur">
                                <span class="d-select" style="width:140px">
                                    <select class="d-input" id="pr-cur" on:change=move |e| patch_me(app, serde_json::json!({ "currency": event_target_value(&e) }))>
                                        {[("inr", "inr, ₹"), ("usd", "usd, $"), ("eur", "eur, €")].into_iter().map(|(v, l)| view! { <option value=v selected=move || currency.get() == v>{l}</option> }).collect_view()}
                                    </select>
                                </span>
                            </SettingRow>
                            <SettingRow label="theme" hint="every pebblelab app on this device">
                                <Segmented value=theme_name label="theme">
                                    <SegOption value="dark" label="dark"><Ico d=MOON/></SegOption>
                                    <SegOption value="light" label="light"><Ico d=SUN/></SegOption>
                                </Segmented>
                            </SettingRow>
                        </SettingRows>
                    </div>
                </section>
                <section class="d-card" aria-labelledby="nt-t">
                    <header class="d-card__head"><h2 class="d-card__title" id="nt-t">"notify me about"</h2></header>
                    <div class="d-card__body">
                        <SettingRows>
                            {switch("card due dates", "a few days before a payment is due", "notify_card", |u| u.notify_card)}
                            {switch("loan emis", "the day an emi leaves", "notify_emi", |u| u.notify_emi)}
                            {move || app.has_family().then(|| switch("transactions on joint accounts", "when someone else adds one", "notify_joint", |u| u.notify_joint))}
                        </SettingRows>
                    </div>
                </section>
                <section class="d-card" aria-labelledby="dx-t">
                    <header class="d-card__head"><h2 class="d-card__title" id="dx-t">"your data"</h2></header>
                    <div class="d-card__body">
                        <SettingRows>
                            <SettingRow label="accounts and transactions" hint="as csv">
                                <button type="button" class="d-btn d-btn--secondary" on:click=export><Ico d=DOWNLOAD/>"export"</button>
                            </SettingRow>
                        </SettingRows>
                    </div>
                </section>
                <section class="d-card" aria-labelledby="ac-t">
                    <header class="d-card__head"><h2 class="d-card__title" id="ac-t">"in your pebblelab account"</h2></header>
                    <div class="d-card__body">
                        <SettingRows>
                            <SettingRow label="profile and security" hint="name, picture, password and where you are signed in">
                                <a class="d-btn d-btn--secondary" href=move || account("profile") rel="external">"open"</a>
                            </SettingRow>
                            <SettingRow label="family" hint="who shares accounts with you">
                                <a class="d-btn d-btn--secondary" href=move || account("family") rel="external">"open"</a>
                            </SettingRow>
                            <SettingRow label="connectors" hint="tokens for claude and other mcp clients">
                                <a class="d-btn d-btn--secondary" href=move || account("connectors") rel="external">"open"</a>
                            </SettingRow>
                        </SettingRows>
                    </div>
                </section>
            </SettingGrid>
        </div>
    }
}
