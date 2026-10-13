//! The tile at the top of every sidebar, in the account and in each app: all switching lives here. Who you are,
//! which app, which household you are looking at, the theme, and signing out.

use dots_ui::prelude::*;
use leptos::prelude::*;

use crate::api;
use crate::apps::{self, APPS, Glyph};
use crate::hub_art::MARK;
use crate::icons::*;
use crate::state::AppState;

#[component]
pub fn Switcher() -> impl IntoView {
    let app = expect_context::<AppState>();
    let here = apps::here();
    let in_account = here == apps::ACCOUNT;
    let hub = api::options().hub;
    // the account's own address, from an app; apps without a hub keep the account in their settings
    let account = if in_account { None } else { Some(hub.clone().map(|h| format!("{h}/account")).unwrap_or_else(|| "/settings".into())) };
    let many = in_account || hub.is_some();
    let title = if in_account { "pebblelab account" } else { apps::find(here).map(|a| a.name).unwrap_or(here) };

    let open = RwSignal::new(false);
    let theme = use_theme();
    let root = NodeRef::<leptos::html::Div>::new();
    // a click anywhere else, or esc, closes it
    let click = window_event_listener(leptos::ev::pointerdown, move |e| {
        use wasm_bindgen::JsCast;
        let inside = match (root.get_untracked(), e.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok())) {
            (Some(r), Some(t)) => r.contains(Some(&t)),
            _ => false,
        };
        if !inside && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || click.remove());

    let name = move || app.me.with(|m| m.as_ref().map(|m| m.user.name.clone()).unwrap_or_default());
    let email = move || app.me.with(|m| m.as_ref().map(|m| m.user.email.clone()).unwrap_or_default());
    let picture = Signal::derive(move || app.me.with(|m| m.as_ref().and_then(|m| m.user.picture.clone())));
    let family = move || app.me.with(|m| m.as_ref().and_then(|m| m.family.clone()));
    // under the name of the place: who you are in the account, what you are looking at in an app
    let sub = move || if in_account { name() } else { app.household() };
    let sign_out = move |_| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/auth/signout", &serde_json::json!({})).await;
            api::clear_token();
            app.me.set(None);
        });
    };

    view! {
        <div node_ref=root on:keydown=move |e| if e.key() == "Escape" { open.set(false) }>
            <button type="button" class="d-sidebar__tile sb__trigger" aria-haspopup="dialog" aria-expanded=move || open.get().to_string()
                aria-label=move || format!("switch: {title}, {}", sub()) on:click=move |_| open.update(|o| *o = !*o)>
                <span class="d-sidebar__mark" aria-hidden="true"><svg viewBox="-3.4 5.6 40 40" width="24" height="24" style="flex:none"><path d=MARK fill="var(--accent-primary)"></path></svg></span>
                <span class="d-sidebar__who"><b>{title}</b><small class=(!in_account).then_some("sb__ctx")>{sub}</small></span>
                <Ico d=UP_DOWN/>
            </button>
            {move || open.get().then(|| view! {
                <div class="d-menu sb" role="dialog" aria-label="switch app, household and account">
                    <div class="sb__sec">
                        <div class="sb__you">
                            <span class="d-avatar d-avatar--lg" aria-hidden="true"><Face picture=picture/></span>
                            <span class="d-menu__who"><b>{name}</b><small>{email}</small></span>
                        </div>
                        {account.clone().map(|href| view! {
                            <a class="sb__row" href=href.clone() rel=href.starts_with("http").then_some("external") on:click=move |_| open.set(false)>
                                <Ico d=USER/><span>"pebblelab account"<small>"profile, sign-in, family, tokens"</small></span>
                            </a>
                        })}
                    </div>
                    {many.then(|| view! {
                        <div class="sb__sec">
                            <div class="sb__cap"><span>"apps"</span><span>"one sign-in"</span></div>
                            <div class="sb__apps">
                                {APPS.iter().map(|a| match (a.id == here, a.url()) {
                                    (true, _) => view! { <span class="d-apps__tile" aria-current="true"><span class="d-apps__mark" aria-hidden="true"><Glyph app=a/></span><span class="d-apps__name">{a.name}</span></span> }.into_any(),
                                    (false, Some(url)) => view! { <a class="d-apps__tile" href=url rel="external"><span class="d-apps__mark" aria-hidden="true"><Glyph app=a/></span><span class="d-apps__name">{a.name}</span></a> }.into_any(),
                                    (false, None) => view! { <span class="d-apps__tile" aria-disabled="true"><span class="d-apps__mark" aria-hidden="true"><Glyph app=a/></span><span class="d-apps__name">{a.name}</span></span> }.into_any(),
                                }).collect_view()}
                                {match account.clone() {
                                    None => view! { <span class="d-apps__tile" aria-current="true"><span class="d-apps__mark" aria-hidden="true"><Ico d=USER/></span><span class="d-apps__name">"account"</span></span> }.into_any(),
                                    Some(href) => view! { <a class="d-apps__tile" href=href rel="external"><span class="d-apps__mark" aria-hidden="true"><Ico d=USER/></span><span class="d-apps__name">"account"</span></a> }.into_any(),
                                }}
                                <span class="d-apps__tile" aria-disabled="true"><span class="d-apps__mark" aria-hidden="true"></span><span class="d-apps__name">"soon"</span></span>
                            </div>
                        </div>
                    })}
                    {move || family().map(|f| {
                        let names = f.members.iter().map(|m| m.name.split(' ').next().unwrap_or("").to_string()).collect::<Vec<_>>().join(" and ");
                        view! {
                            <div class="sb__sec" role="radiogroup" aria-label="household">
                                {if in_account {
                                    view! { <div class="sb__cap"><span>"household"</span><span>"every app follows it"</span></div> }
                                } else {
                                    view! { <div class="sb__cap"><span>"looking at"</span><span>"accounts, insights, people"</span></div> }
                                }}
                                <button type="button" class="sb__row" role="radio" aria-checked=move || (!app.solo.get()).to_string() on:click=move |_| app.set_solo(false)>
                                    <span class="sb__radio"></span><span>{f.name.clone()}<small>{names}</small></span>
                                </button>
                                <button type="button" class="sb__row" role="radio" aria-checked=move || app.solo.get().to_string() on:click=move |_| app.set_solo(true)>
                                    <span class="sb__radio"></span><span>"just you"<small>"only what you own"</small></span>
                                </button>
                            </div>
                        }
                    })}
                    <div class="sb__sec" style="display:flex;align-items:center;justify-content:space-between;gap:8px">
                        <div class="d-seg" role="radiogroup" aria-label="theme" style="width:120px">
                            <button type="button" class="d-seg__opt" role="radio" aria-checked=move || (theme.get() == Theme::Dark).to_string() aria-label="dark" on:click=move |_| theme.set(Theme::Dark)><Ico d=MOON/></button>
                            <button type="button" class="d-seg__opt" role="radio" aria-checked=move || (theme.get() == Theme::Light).to_string() aria-label="light" on:click=move |_| theme.set(Theme::Light)><Ico d=SUN/></button>
                        </div>
                        <button type="button" class="d-btn d-btn--ghost" on:click=sign_out><Ico d=SIGN_OUT/>"sign out"</button>
                    </div>
                </div>
            })}
        </div>
    }
}
