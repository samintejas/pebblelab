//! The tile at the top of each sidebar, and what it opens: who you are, the pebblelab apps, the household,
//! the theme and signing out. Fin and the account (pebblelab.in) draw the same one, so switching looks the
//! same wherever you are.

use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::api;
use crate::hub_art::MARK;
use crate::icons::*;
use crate::state::AppState;

/// Which app the switcher sits in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Here {
    Fin,
    Account,
}

/// The account's address, from fin: the hub when it is in use, else fin's own settings.
fn account_href(page: &str) -> (String, bool) {
    match api::options().hub {
        Some(h) => (format!("{}/{page}", h.trim_end_matches('/')), true),
        None => (if page == "family" { "/settings/family".into() } else { "/settings".into() }, false),
    }
}

/// Fin's address, from the account.
fn fin_href() -> String {
    api::options().fin.unwrap_or_else(|| "/".into())
}

#[component]
pub fn Switcher(here: Here, #[prop(into)] on_sign_out: Callback<()>) -> impl IntoView {
    let app = expect_context::<AppState>();
    let open = RwSignal::new(false);

    // a click anywhere else, esc, or going to another page closes it
    let root = NodeRef::<leptos::html::Div>::new();
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
    let path = use_location().pathname;
    Effect::new(move |_| {
        path.track();
        open.set(false);
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

    let name = move || app.me.with(|m| m.as_ref().map(|m| m.user.name.clone()).unwrap_or_default());
    let email = move || app.me.with(|m| m.as_ref().map(|m| m.user.email.clone()).unwrap_or_default());
    let picture = Signal::derive(move || app.me.with(|m| m.as_ref().and_then(|m| m.user.picture.clone())));
    let household = move || app.me.with(|m| m.as_ref().and_then(|m| m.family.as_ref().map(|f| f.name.clone()))).unwrap_or_else(|| "just you".into());
    let members = move || {
        app.me.with(|m| match m.as_ref().and_then(|m| m.family.as_ref()) {
            Some(f) if f.members.len() > 1 => f.members.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(" and "),
            Some(_) => "nobody else has joined yet".into(),
            None => "no family yet. add one to share with the people you choose.".into(),
        })
    };

    let (title, sub) = match here {
        Here::Fin => ("fin", Signal::derive(household)),
        Here::Account => ("pebblelab account", Signal::derive(name)),
    };
    let (account, account_ext) = account_href("account");
    let (family, family_ext) = match here {
        Here::Account => ("/family".to_string(), false),
        Here::Fin => account_href("family"),
    };

    view! {
        <div node_ref=root on:keydown=move |e| if e.key() == "Escape" && open.get_untracked() { open.set(false) }>
            <button type="button" class="d-sidebar__tile" aria-haspopup="dialog"
                aria-expanded=move || open.get().to_string()
                aria-label=move || format!("switch: {title}, {}", sub.get())
                on:click=move |_| open.update(|o| *o = !*o)>
                <span class="d-sidebar__mark" aria-hidden="true"><svg viewBox="-3.4 5.6 40 40" width="24" height="24" style="flex:none"><path d=MARK fill="var(--accent-primary)"></path></svg></span>
                <span class="d-sidebar__who"><b>{title}</b><small class:sb__ctx=here == Here::Fin>{move || sub.get()}</small></span>
                <Ico d=UP_DOWN/>
            </button>
            {move || open.get().then(|| {
                let (account, family) = (account.clone(), family.clone());
                view! {
                    <div class="d-menu sb" role="dialog" aria-label="switch app, household and account">
                        <div class="sb__sec">
                            <div class="sb__you">
                                <span class="d-avatar d-avatar--lg" aria-hidden="true"><Face picture=picture/></span>
                                <span class="d-menu__who"><b>{name}</b><small>{email}</small></span>
                            </div>
                            {(here == Here::Fin).then(|| view! {
                                <a class="sb__row" href=account.clone() rel=account_ext.then_some("external")>
                                    <Ico d=USER/><span>"pebblelab account"<small>"profile, sign-in, family, tokens"</small></span>
                                </a>
                            })}
                        </div>
                        <div class="sb__sec">
                            <div class="sb__cap"><span>"apps"</span><span>"one sign-in"</span></div>
                            <div class="sb__apps">
                                {match here {
                                    Here::Fin => view! { <span class="d-apps__tile" aria-current="true"><span class="d-apps__mark" aria-hidden="true"><Ico d=WALLET/></span><span class="d-apps__name">"fin"</span></span> }.into_any(),
                                    Here::Account => view! { <a class="d-apps__tile" href=fin_href() rel="external"><span class="d-apps__mark" aria-hidden="true"><Ico d=WALLET/></span><span class="d-apps__name">"fin"</span></a> }.into_any(),
                                }}
                                {match here {
                                    Here::Account => view! { <span class="d-apps__tile" aria-current="true"><span class="d-apps__mark" aria-hidden="true"><Ico d=USER/></span><span class="d-apps__name">"account"</span></span> }.into_any(),
                                    Here::Fin => view! { <a class="d-apps__tile" href=account rel=account_ext.then_some("external")><span class="d-apps__mark" aria-hidden="true"><Ico d=USER/></span><span class="d-apps__name">"account"</span></a> }.into_any(),
                                }}
                            </div>
                        </div>
                        <div class="sb__sec">
                            <div class="sb__cap"><span>"household"</span><span>"every app follows it"</span></div>
                            <a class="sb__row" href=family rel=family_ext.then_some("external")>
                                <Ico d=USERS/><span>{household}<small>{members}</small></span>
                            </a>
                        </div>
                        <div class="sb__sec sb__foot">
                            <Segmented value=theme_name label="theme">
                                <SegOption value="system" aria_label="follow the system"><Ico d=MONITOR/></SegOption>
                                <SegOption value="dark" aria_label="dark"><Ico d=MOON/></SegOption>
                                <SegOption value="light" aria_label="light"><Ico d=SUN/></SegOption>
                            </Segmented>
                            <button type="button" class="d-btn d-btn--ghost" on:click=move |_| { open.set(false); on_sign_out.run(()); }><Ico d=SIGN_OUT/>"sign out"</button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
