//! The apps under pebblelab. One account, one family and one set of connectors serve all of them; each app is
//! a row here, and the landing page, the account's home and the switcher at the top of every sidebar are drawn
//! from this list. A new app is a new row (and its screens), nothing else.

use leptos::prelude::*;

use crate::api;

/// The account itself, where an app is named: the session's `app`, the switcher's current tile.
pub const ACCOUNT: &str = "account";

pub struct AppDef {
    /// What the server and the address bar call it: `?to=fin`, the session's `app`.
    pub id: &'static str,
    pub name: &'static str,
    /// One line, for the lists of apps.
    pub blurb: &'static str,
    /// Its address as people read it.
    pub host: &'static str,
    /// Its glyph: stroke paths on a 24px viewbox.
    pub glyph: &'static [&'static str],
}

pub const APPS: [AppDef; 1] = [AppDef {
    id: "fin",
    name: "fin",
    blurb: "a household money tracker. accounts, loans, subscriptions, and a family that shares them.",
    host: "fin.pebblelab.in",
    glyph: &[crate::hub_art::FIN, "M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4"],
}];

pub fn find(id: &str) -> Option<&'static AppDef> {
    APPS.iter().find(|a| a.id == id)
}

impl AppDef {
    /// Where it lives on this server, when the server has been told.
    pub fn url(&self) -> Option<String> {
        match self.id {
            "fin" => api::options().fin,
            _ => None,
        }
    }
}

/// Which app this page is: decided by the address it was opened at.
pub fn here() -> &'static str {
    if api::is_hub() { ACCOUNT } else { "fin" }
}

/// An app's glyph, drawn at the size of whatever holds it.
#[component]
pub fn Glyph(app: &'static AppDef) -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {app.glyph.iter().map(|d| view! { <path d=*d></path> }).collect_view()}
        </svg>
    }
}
