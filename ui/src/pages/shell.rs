use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};

use crate::api;
use crate::icons::*;
use crate::pages::transactions::TxPanel;
use crate::pages::switcher::{Here, Switcher};
use crate::pages::{AssetPanel, SubPanel};
use crate::state::{AppState, Side};

/// `/accounts/3` and `settings/family` style links from a notification.
fn link_path(link: &str) -> Option<String> {
    (!link.is_empty()).then(|| format!("/{}", link.trim_start_matches('/')))
}

/// `2026-10-05 12:30:00` (utc) to `today`, `yesterday` or `05 oct`.
fn when(created: &str) -> String {
    let day = created.get(..10).unwrap_or(created);
    if day == crate::fmt::today() {
        "today".into()
    } else if day == crate::fmt::days_ago(1) {
        "yesterday".into()
    } else {
        crate::fmt::day(day)
    }
}

/// The signed-in frame: sidebar, app bar, the page, and the side panel. Redirects to sign in without a
/// token, and loads the shared data once.
#[component]
pub fn AppShell() -> impl IntoView {
    let app = AppState::new();
    provide_context(app);
    let nav = use_navigate();
    if api::load_token().is_none() {
        nav("/signin", Default::default());
    } else {
        app.reload();
    }
    {
        let nav = nav.clone();
        Effect::new(move |_| {
            if app.me.get().is_none() && api::token().is_none() {
                nav("/signin", Default::default());
            }
        });
    }
    let shell = ShellState::new();
    // the side panel is open exactly while a transaction, subscription or asset is
    Effect::new(move |_| shell.set(Region::Right, app.panel.get().is_some() || app.side.get().is_some()));

    let path = use_location().pathname;
    let page = Memo::new(move |_| path.get().trim_start_matches('/').split('/').next().unwrap_or("").to_string());
    // leaving a screen closes its panel
    Effect::new(move |_| {
        let p = page.get();
        if p != "transactions" {
            app.panel.set(None);
        }
        let keep = app.side.with_untracked(|s| match s {
            Some(Side::Sub(_)) => p == "subscriptions",
            Some(Side::Asset(_)) => p == "assets",
            None => true,
        });
        if !keep {
            app.side.set(None);
        }
    });
    let crumb = move || {
        let p = path.get();
        let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
        match parts.as_slice() {
            ["settings"] => "settings / profile".to_string(),
            ["settings", "prefs"] => "settings / preferences".to_string(),
            ["settings", tab] => format!("settings / {tab}"),
            ["accounts", "new"] => "accounts / new".to_string(),
            ["accounts", id] => format!("accounts / {}", id.parse().map(|i| app.account_name(i)).unwrap_or_default()),
            [one] => one.to_string(),
            _ => String::new(),
        }
    };

    let sign_out = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/auth/signout", &serde_json::json!({})).await;
            api::clear_token();
            app.me.set(None);
        });
    });
    let acct_count = move || app.accounts.with(|a| a.iter().filter(|a| !a.archived).count());

    view! {
        <SkipLink/>
        <Shell state=shell height="100dvh">
            <Sidebar>
                <SidebarHead><Switcher here=Here::Fin on_sign_out=sign_out/></SidebarHead>
                <SidebarContent>
                    <SidebarGroup label="money">
                        <SidebarItem href="/transactions" icon="arrow-left-right" label="transactions" current=Signal::derive(move || page.get() == "transactions")/>
                        <SidebarItem href="/insights" icon="layout-dashboard" label="insights" current=Signal::derive(move || page.get() == "insights")/>
                        {move || view! { <SidebarItem href="/accounts" icon="wallet" label="accounts" current=Signal::derive(move || page.get() == "accounts") count=acct_count().to_string()/> }}
                    </SidebarGroup>
                    <SidebarGroup label="track">
                        <TrackItem href="/subscriptions" icon=REPEAT label="subscriptions" current=Signal::derive(move || page.get() == "subscriptions")/>
                        <TrackItem href="/assets" icon=PACKAGE label="assets" current=Signal::derive(move || page.get() == "assets")/>
                    </SidebarGroup>
                </SidebarContent>
                <SidebarFoot>
                    <ul class="d-sidebar__menu">
                        <TrackItem href="/settings" icon=SLIDERS label="settings" current=Signal::derive(move || page.get() == "settings")/>
                    </ul>
                </SidebarFoot>
            </Sidebar>
            <ShellMain>
                <AppBar>
                    <SidebarToggle/>
                    <AppBarSep/>
                    <Breadcrumbs>
                        <Crumb href="/insights">{move || app.root_name()}</Crumb>
                        <CrumbCurrent>{crumb}</CrumbCurrent>
                    </Breadcrumbs>
                    <AppBarSpacer/>
                    <Notifications/>
                </AppBar>
                <Main><div class="page"><Outlet/></div></Main>
                {move || shell.is_mobile().then(|| view! { <TabBar page=page/> })}
            </ShellMain>
            // on a phone the sidebar is a drawer over the page: a tap beside it closes it
            {move || (shell.is_mobile() && shell.is_open(Region::Left)).then(|| view! { <div class="m-scrim" aria-hidden="true" on:click=move |_| shell.set(Region::Left, false)></div> })}
            <aside class="d-rightbar" aria-label="details" inert=move || app.panel.get().is_none() && app.side.get().is_none()>
                {move || app.panel.get().map(|t| view! { <TxPanel t=t/> })}
                {move || app.side.get().map(|s| match s {
                    Side::Sub(x) => view! { <SubPanel sub=x/> }.into_any(),
                    Side::Asset(x) => view! { <AssetPanel asset=x/> }.into_any(),
                })}
            </aside>
        </Shell>
    }
}

/// On a phone: the screens used most, a thumb away, and the rest of the sidebar behind "more".
#[component]
fn TabBar(page: Memo<String>) -> impl IntoView {
    let shell = use_shell();
    let tab = move |href: &'static str, icon: &'static str, label: &'static str| {
        let key = href.trim_start_matches('/');
        view! {
            <a class="m-tab" href=href aria-current=move || (page.get() == key).then_some("page")>
                <Ico d=icon/><span>{label}</span>
            </a>
        }
    };
    let more_current = move || ["subscriptions", "assets", "settings"].contains(&page.get().as_str());
    view! {
        <nav class="m-tabs" aria-label="screens">
            {tab("/transactions", SWAP, "transactions")}
            {tab("/insights", DASHBOARD, "insights")}
            {tab("/accounts", WALLET, "accounts")}
            <button type="button" class="m-tab" aria-current=move || more_current().then_some("page")
                aria-expanded=move || shell.is_open(Region::Left).to_string() on:click=move |_| shell.toggle(Region::Left)>
                <Ico d=MENU/><span>"more"</span>
            </button>
        </nav>
    }
}

/// A sidebar row for the screens whose icon is not in the bundled set: the same markup as `SidebarItem`.
#[component]
fn TrackItem(href: &'static str, icon: &'static str, label: &'static str, #[prop(into)] current: Signal<bool>) -> impl IntoView {
    let shell = use_shell();
    view! {
        <li class="d-sidebar__item">
            <a class="d-sidebar__btn" href=href aria-current=move || current.get().then_some("page") data-tooltip=move || shell.icon_rail().then_some(label)>
                <Ico d=icon/>
                <span class="d-sidebar__label">{label}</span>
            </a>
        </li>
    }
}

/// The bell and its panel: a title, one line of detail and when, with a dot on what is unread.
#[component]
fn Notifications() -> impl IntoView {
    let app = expect_context::<AppState>();
    let open = RwSignal::new(false);
    let nav = use_navigate();
    let unread = move || app.notes.with(|n| n.iter().filter(|n| !n.read).count());
    let refresh = move || {
        leptos::task::spawn_local(async move {
            if let Ok(n) = api::notifications().await {
                app.notes.set(n);
            }
        });
    };
    let read = move |id: Option<i64>| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/notifications/read", &serde_json::json!({ "id": id })).await;
            refresh();
        });
    };
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
    let label = move || match unread() {
        0 => "notifications".to_string(),
        n => format!("notifications, {n} unread"),
    };
    let nav2 = nav.clone();
    view! {
        <div class="d-appbar__actions" style="position:relative;margin-right:8px" node_ref=root on:keydown=move |e| if e.key() == "Escape" { open.set(false) }>
            <button type="button" class="d-btn d-btn--ghost d-btn--icon d-btn--sm" aria-label=label aria-haspopup="true"
                aria-expanded=move || open.get().to_string() on:click=move |_| open.update(|o| *o = !*o)>
                <Ico d=BELL/>
                {move || { let n = unread(); (n > 0).then(|| view! { <span class="d-btn__count" aria-hidden="true">{if n > 9 { "9+".to_string() } else { n.to_string() }}</span> }) }}
            </button>
            {move || open.get().then(|| {
                let (nav, nav2) = (nav.clone(), nav2.clone());
                view! {
                    <section class="d-menu" aria-label="notifications" style="position:absolute;top:calc(100% + 8px);right:0;z-index:200;width:360px;max-width:calc(100vw - 32px);padding:0;overflow:hidden">
                        <div class="d-row" style="justify-content:space-between;flex-wrap:nowrap;min-height:44px;padding:8px 8px 8px 16px">
                            <span style="color:var(--text-strong);font-weight:500">"notifications"</span>
                            {move || (unread() > 0).then(|| view! { <button type="button" class="d-btn d-btn--ghost d-btn--sm" on:click=move |_| read(None)>"mark all read"</button> })}
                        </div>
                        <div style="max-height:min(420px, 60vh);overflow-y:auto">
                        {move || {
                            let list = app.notes.get();
                            if list.is_empty() {
                                return view! { <div style="padding:12px 16px;border-top:1px solid var(--border-subtle);color:var(--text-secondary)">"nothing yet. card due dates, emis and family activity show up here."</div> }.into_any();
                            }
                            view! { <ul class="d-feed" aria-label="notifications">{list.into_iter().map(|n| {
                                let nav = nav.clone();
                                let (id, link, unread) = (n.id, link_path(&n.link), !n.read);
                                view! {
                                    <li><button type="button" class="d-feed__item" data-unread=unread.then_some("") on:click=move |_| {
                                        if unread { read(Some(id)); }
                                        open.set(false);
                                        if let Some(l) = &link { nav(l, Default::default()); }
                                    }>
                                        <span class="d-feed__mark" aria-hidden="true"></span>
                                        <span class="d-feed__body">
                                            <span class="d-feed__title">{unread.then(|| view! { <span class="d-sr">"unread: "</span> })}{n.title.clone()}</span>
                                            <span class="d-feed__text">{n.body.clone()}</span>
                                        </span>
                                        <span class="d-feed__time">{when(&n.created_at)}</span>
                                    </button></li>
                                }
                            }).collect_view()}</ul> }.into_any()
                        }}
                        </div>
                        <div class="d-row" style="justify-content:flex-end;flex-wrap:nowrap;padding:8px 8px 8px 16px;border-top:1px solid var(--border-subtle)">
                            <button type="button" class="d-btn d-btn--ghost d-btn--sm" on:click=move |_| { open.set(false); nav2("/settings/prefs", Default::default()); }>"notification settings"</button>
                        </div>
                    </section>
                }
            })}
        </div>
    }
}
