//! The switchboard at the top of every sidebar: who you are, which app, your family, the theme, signing out.
//! It is the only place anything switches.

use dots_ui::prelude::*;
use leptos::prelude::*;

use crate::api;
use crate::icons::{Face, Mark};
use crate::site::{self, use_site};
use crate::state::AppState;

/// Where an app opens.
fn home(app: &str) -> &'static str {
    match app {
        "account" => "/account",
        "fin" => "/transactions",
        _ => "/",
    }
}

fn icon_for(app: &str) -> &'static str {
    match app {
        "fin" => "wallet",
        _ => "user",
    }
}

/// `current` is the app this page is (`fin`, `account`); `extra` goes above the account rows (fin settings).
#[component]
pub fn Switcher(current: &'static str, #[prop(optional, into)] extra: Option<ViewFn>) -> impl IntoView {
    let app = expect_context::<AppState>();
    let site = use_site();
    let theme = use_theme();
    let theme_name = RwSignal::new(theme.get_untracked().as_str().to_string());
    Effect::new(move |_| {
        let t = Theme::parse(&theme_name.get()).unwrap_or_default();
        if theme.get_untracked() != t {
            theme.set(t);
        }
    });
    Effect::new(move |_| theme_name.set(theme.get().as_str().to_string()));

    let name = move || app.me.with(|m| m.as_ref().map(|m| m.user.name.clone()).unwrap_or_default());
    let email = move || app.me.with(|m| m.as_ref().map(|m| m.user.email.clone()).unwrap_or_default());
    let picture = Signal::derive(move || app.me.with(|m| m.as_ref().and_then(|m| m.user.picture.clone())));
    // the tile says whose money this is: the family's name, or yours
    let title = Signal::derive(move || app.me.with(|m| match m {
        Some(m) => m.family.as_ref().map(|f| f.name.clone()).unwrap_or_else(|| m.user.name.clone()),
        None => String::new(),
    }));
    let initials = Signal::derive(move || title.get().chars().take(2).collect::<String>());
    let account = move |section: &str| site.account_href(&format!("/account{section}"));
    let sign_out = move |()| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/auth/signout", &serde_json::json!({})).await;
            api::clear_token();
            site::go_external("/");
        });
    };
    let app_icon = icon_for(current);
    view! {
        <Switchboard initials=initials title=title app=current app_icon=app_icon mark=|| view! { <Mark size=24/> }>
            <MenuHead>
                <span class="d-avatar" aria-hidden="true"><Face picture=picture/></span>
                <span class="d-menu__who"><b>{name}</b><small>{email}</small></span>
            </MenuHead>
            {move || {
                let s = site.get();
                let tiles = s.apps.iter().map(|a| {
                    let href = site.app_href(&a.id, home(&a.id));
                    view! { <AppTile href=href name=a.name.clone() icon=icon_for(&a.id) current=a.id == current/> }
                }).collect_view();
                view! { <SwitchboardApps>{tiles}<AppTile href="" name="notes" icon="scroll-text" disabled=true/></SwitchboardApps> }
            }}
            {extra.map(|e| e.run())}
            <MenuLink href=account("/family")>"family"</MenuLink>
            <MenuLink href=account("")>"manage your pebblelab account"</MenuLink>
            <MenuSeparator/>
            <MenuRadioGroup label="theme" value=theme_name>
                <MenuRadio value="dark">"dark"</MenuRadio>
                <MenuRadio value="light">"light"</MenuRadio>
            </MenuRadioGroup>
            <MenuSeparator/>
            <MenuItem on_select=sign_out>"sign out"</MenuItem>
        </Switchboard>
    }
}
