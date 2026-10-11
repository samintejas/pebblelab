//! The app switcher in every app bar: the account and each app, wherever they live.

use dots_ui::prelude::*;
use leptos::prelude::*;

use crate::icons::Mark;
use crate::site::use_site;

/// Where an app opens.
fn home(app: &str) -> &'static str {
    match app {
        "account" => "/account",
        "fin" => "/transactions",
        _ => "/",
    }
}

/// The `grip` button and its grid. `current` is the app this page is.
#[component]
pub fn AppsMenu(current: &'static str) -> impl IntoView {
    let site = use_site();
    move || {
        let s = site.get();
        let account = site.app_href("account", "/account");
        let tiles = s
            .apps
            .iter()
            .map(|a| {
                let href = site.app_href(&a.id, home(&a.id));
                let mark = (a.id == "fin").then(|| view! { <Mark/> }.into_any());
                let icon = match a.icon.as_str() {
                    "wallet" => "wallet",
                    _ => "user",
                };
                match mark {
                    Some(m) => view! { <AppTile href=href name=a.name.clone() current=a.id == current>{m}</AppTile> }.into_any(),
                    None => view! { <AppTile href=href name=a.name.clone() icon=icon current=a.id == current/> }.into_any(),
                }
            })
            .collect_view();
        view! {
            <AppSwitcher label="pebblelab apps" foot=move || view! { <MenuLink href=account.clone()>"manage your pebblelab account"</MenuLink> }>
                {tiles}
            </AppSwitcher>
        }
    }
}
