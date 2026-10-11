//! Whole screens for when there is no screen to show: an address that is not a page, and opening an app.

use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// Any address no route matches.
#[component]
pub fn NotFound() -> impl IntoView {
    let path = use_location().pathname;
    view! {
        <StatusPage
            kind=StatusKind::NotFound
            title="nothing at this address"
            foot="pebblelab"
            actions=|| view! {
                <ButtonLink href="/" variant=ButtonVariant::Primary size=Size::Lg>"go home"</ButtonLink>
                <button type="button" class="d-btn d-btn--secondary d-btn--lg" on:click=|_| { let _ = window().history().and_then(|h| h.back()); }>"go back"</button>
            }
        >
            <code>{move || path.get()}</code>" is not a page here. check the address, or start again from home."
        </StatusPage>
    }
}

/// While an app finds out who is signed in and loads what it needs.
#[component]
pub fn Opening(#[prop(into)] app: String) -> impl IntoView {
    let title = format!("opening {app}");
    view! { <StatusPage kind=StatusKind::Loading title=title/> }
}
