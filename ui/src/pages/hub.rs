//! pebblelab.in: the account every pebblelab app signs in through. Landing, sign in with Google or GitHub (no
//! passwords), and the account itself: profile, sign-in and security, family, connectors.

use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::path;
use pebblelab_api::*;

use super::settings::{Connectors, FamilyTab, ProfileTab};
use crate::api;
use crate::hub_art::{FIN, MARK, Wordmark};
use crate::icons::*;
use crate::pages::switcher::{Here, Switcher};
use crate::state::AppState;

const TO_KEY: &str = "pebblelab:to";

fn remember_to(to: Option<&str>) {
    if let Some(s) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        let _ = match to {
            Some(t) => s.set_item(TO_KEY, t),
            None => s.remove_item(TO_KEY),
        };
    }
}

fn take_to() -> Option<String> {
    let s = web_sys::window()?.session_storage().ok().flatten()?;
    let v = s.get_item(TO_KEY).ok().flatten();
    let _ = s.remove_item(TO_KEY);
    v
}

#[component]
pub fn HubApp() -> impl IntoView {
    view! {
        <DotsStyles font_base="/fonts/"/>
        <style>{include_str!("../app.css")}</style>
        <DotsProvider persist_theme="pebblelab:theme">
            <Router>
                <Routes fallback=|| view! { <StatusPage kind=StatusKind::NotFound title="you are lost" foot="pebblelab"><a class="d-link" href="/">"click here to go home"</a></StatusPage> }>
                    <Route path=path!("/") view=Landing/>
                    <Route path=path!("/signin") view=SignIn/>
                    <Route path=path!("/auth/callback") view=Callback/>
                    <ParentRoute path=path!("") view=HubShell>
                        <Route path=path!("account") view=Home/>
                        <Route path=path!("profile") view=Profile/>
                        <Route path=path!("security") view=Security/>
                        <Route path=path!("family") view=Family/>
                        <Route path=path!("connectors") view=ConnectorsPage/>
                    </ParentRoute>
                </Routes>
            </Router>
        </DotsProvider>
    }
}

#[component]
fn FinGlyph() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d=FIN></path><path d="M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4"></path>
        </svg>
    }
}

fn fin_href() -> String {
    api::options().fin.unwrap_or_else(|| "/".into())
}

/// Google and GitHub, nothing else. A full page load: the provider is another site.
#[component]
fn ProviderButtons() -> impl IntoView {
    let list = LocalResource::new(api::providers);
    view! {
        {move || list.get().map(|l| l.into_iter().map(|p| {
            let (mark, name) = if p == "github" { ("gh", "github") } else { ("g", "google") };
            view! {
                <a class="pl-provider" rel="external" href=format!("/api/auth/{p}/start")><b aria-hidden="true">{mark}</b>{format!("continue with {name}")}</a>
            }
        }).collect_view())}
    }
}

#[component]
fn Landing() -> impl IntoView {
    let nav = use_navigate();
    if api::load_token().is_some() {
        nav("/account", Default::default());
    }
    view! {
        <SkipLink/>
        <div class="pl-page">
            <AppBar>
                <span style="display:flex;margin-left:4px"><Wordmark/></span>
                <AppBarSpacer/>
                <ButtonLink href="/signin" variant=ButtonVariant::Secondary size=Size::Sm>"sign in"</ButtonLink>
            </AppBar>
            <main id="main" tabindex="-1" style="flex:1">
                <section class="pl-hero">
                    <div class="d-container" style="padding-block:96px 72px;display:flex;flex-wrap:wrap;gap:48px;align-items:center;justify-content:space-between">
                        <div style="flex:1 1 420px;min-width:0;display:flex;flex-direction:column;gap:20px;max-width:600px">
                            <h1 class="heading-3xl" style="margin:0;color:var(--text-max);max-width:18ch">"small, simple apps. one account."</h1>
                            <p style="margin:0;color:var(--text-secondary);font-size:var(--font-size-lg);max-width:52ch">"sign in once with google or github. every pebblelab app then knows who you are, who your family is, and which ai clients you let in. there is no password to make or forget."</p>
                            <div style="display:flex;flex-direction:column;gap:8px;max-width:340px">
                                <ProviderButtons/>
                                <span style="color:var(--text-secondary);font-size:var(--font-size-xs)">"new here? the same buttons make your account."</span>
                            </div>
                        </div>
                        <div aria-hidden="true" style="flex:0 1 280px;display:flex;justify-content:center;padding-bottom:24px">
                            <svg viewBox="-3.4 5.6 40 40" width="168" height="168"><path d=MARK fill="var(--accent-primary)"></path></svg>
                        </div>
                    </div>
                </section>
                <section class="d-container" style="padding-bottom:72px;display:flex;flex-direction:column;gap:12px;max-width:880px" aria-labelledby="pl-apps">
                    <h2 class="heading-lg" id="pl-apps" style="margin:0;color:var(--text-strong)">"the apps"</h2>
                    <a class="pl-app" href=fin_href() rel="external">
                        <span class="pl-mark" aria-hidden="true"><FinGlyph/></span>
                        <span style="display:flex;flex-direction:column;gap:2px;min-width:0"><b style="color:var(--text-strong);font-weight:500">"fin"</b><span style="color:var(--text-secondary)">"a household money tracker. accounts, loans, subscriptions, and a family that shares them."</span></span>
                        <span style="color:var(--text-secondary);font-size:var(--font-size-sm)">"fin.pebblelab.in"</span>
                    </a>
                    <Soon d=BOX name="shelf" text="a home inventory. what you have, where it is, what is running low. yours, or the household's."/>
                    <Soon d=CLOCK name="hours" text="time, in blocks. a short list, focus, and chores that take turns."/>
                    <Soon d=PLATE name="plate" text="what you eat, against your own targets. a meal plan for the household."/>
                    <div class="pl-app" aria-disabled="true">
                        <span class="pl-mark" aria-hidden="true" style="border-style:dashed"></span>
                        <span style="display:flex;flex-direction:column;gap:2px"><b style="font-weight:500">"more, later"</b><span>"each new app uses the same account. nothing to sign up for again."</span></span>
                        <span style="font-size:var(--font-size-sm)">"soon"</span>
                    </div>
                </section>
            </main>
            <footer style="border-top:1px solid var(--border-default)">
                <div class="d-container d-row" style="padding-block:16px;justify-content:space-between;color:var(--text-secondary);font-size:var(--font-size-sm);align-items:center">
                    <Wordmark width=89 fill="var(--text-secondary)"/>
                    <span>"pebblelab.in"</span>
                </div>
            </footer>
        </div>
    }
}

/// An app that is drawn but not open yet: its glyph, what it is for, and `soon`.
#[component]
fn Soon(d: &'static str, name: &'static str, text: &'static str) -> impl IntoView {
    view! {
        <div class="pl-app" aria-disabled="true">
            <span class="pl-mark" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d=d></path></svg></span>
            <span style="display:flex;flex-direction:column;gap:2px;min-width:0"><b style="font-weight:500">{name}</b><span>{text}</span></span>
            <span style="font-size:var(--font-size-sm)">"soon"</span>
        </div>
    }
}

/// Sign in with google or github. Coming from an app (`?to=fin`) it carries on to that app afterwards, at once
/// when someone is already signed in here.
#[component]
fn SignIn() -> impl IntoView {
    let q = use_query_map();
    let to = move || q.with(|q| q.get("to")).filter(|t| t == "fin");
    let nav = use_navigate();
    let err = RwSignal::new(api::hash_params().remove("error"));
    let busy = RwSignal::new(false);
    remember_to(to().as_deref());
    if api::load_token().is_some() && err.get_untracked().is_none() {
        if to().is_some() {
            busy.set(true);
            leptos::task::spawn_local(async move {
                if let Err(e) = api::open_fin().await {
                    // the session here is stale: sign in again
                    if e.unauthorized() {
                        api::clear_token();
                    }
                    err.set(Some(e.message));
                    busy.set(false);
                }
            });
        } else {
            nav("/account", Default::default());
        }
    }
    view! {
        <SkipLink/>
        <div class="pl-page">
            <AppBar>
                <A href="/" attr:class="d-appbar__title" attr:aria-label="pebblelab" attr:style="display:flex;align-items:center;margin-left:4px"><Wordmark/></A>
            </AppBar>
            {move || if busy.get() {
                view! {
                    <main id="main" tabindex="-1" class="d-status-page" data-kind="loading" style="min-height:calc(100vh - 33px)">
                        <div class="d-street" aria-hidden="true"><div class="d-street__world"><i></i><i></i><i></i></div></div>
                        <div class="d-status-page__inner">
                            <Pebbles label="opening fin" large=true/>
                            <h1 class="d-status-page__title">"opening fin"</h1>
                        </div>
                        <div class="d-status-page__foot">"pebblelab"</div>
                    </main>
                }.into_any()
            } else {
                view! {
                    <main id="main" tabindex="-1" class="d-status-page" data-kind="loading" style="min-height:calc(100vh - 33px)">
                        <div class="d-card" style="width:100%;max-width:380px;text-align:left">
                            <div class="d-card__body" style="padding:24px;display:flex;flex-direction:column;gap:16px">
                                <div style="display:flex;flex-direction:column;gap:4px">
                                    <h1 class="heading-xl" style="margin:0;color:var(--text-strong)">"sign in"</h1>
                                    {move || if to().is_some() {
                                        view! { <p style="margin:0;color:var(--text-secondary)">"to continue to "<b style="color:var(--text-strong);font-weight:500">"fin"</b>"."</p> }.into_any()
                                    } else {
                                        view! { <p style="margin:0;color:var(--text-secondary)">"to your pebblelab account."</p> }.into_any()
                                    }}
                                </div>
                                <ProviderButtons/>
                                <p style="margin:0;color:var(--text-secondary);font-size:var(--font-size-sm)">"no passwords here. you sign in with an account you already have, and the first time makes your pebblelab account."</p>
                                {move || err.get().map(|m| view! { <span class="d-error" role="alert">{format!("error: {m}")}</span> })}
                            </div>
                        </div>
                    </main>
                }.into_any()
            }}
        </div>
    }
}

/// Where a provider sends the person back: swap the one-time code for a session, then on to the app they
/// came from, or their account.
#[component]
fn Callback() -> impl IntoView {
    let err = RwSignal::new(None::<String>);
    let nav = use_navigate();
    let to = take_to();
    leptos::task::spawn_local(async move {
        let Some(code) = api::hash_params().remove("code") else {
            return err.set(Some("there is no sign-in to finish: start again".into()));
        };
        match api::post::<Session>("/auth/redeem", &serde_json::json!({ "code": code, "app": "account" })).await {
            Ok(s) => {
                api::save_token(&s.token, true);
                if to.as_deref() == Some("fin") {
                    if let Err(e) = api::open_fin().await {
                        err.set(Some(e.message));
                    }
                } else {
                    nav("/account", Default::default());
                }
            }
            Err(e) => err.set(Some(if e.unauthorized() { "this sign-in expired or was already used: start again".into() } else { e.message })),
        }
    });
    view! {
        <SkipLink/>
        {move || match err.get() {
            Some(m) => view! {
                <main id="main" tabindex="-1" class="d-status-page" data-kind="error">
                    <div class="d-street" aria-hidden="true"><div class="d-street__world"><i></i><i></i><i></i></div></div>
                    <div class="d-status-page__inner">
                        <h1 class="d-status-page__title">"could not sign you in"</h1>
                        <p class="d-status-page__text"><span class="d-error" role="alert">{format!("error: {m}")}</span></p>
                        <div class="d-status-page__actions"><ButtonLink href="/signin" variant=ButtonVariant::Primary>"back to sign in"</ButtonLink></div>
                    </div>
                    <div class="d-status-page__foot">"pebblelab"</div>
                </main>
            }.into_any(),
            None => view! {
                <main id="main" tabindex="-1" class="d-status-page" data-kind="loading">
                    <div class="d-street" aria-hidden="true"><div class="d-street__world"><i></i><i></i><i></i></div></div>
                    <div class="d-status-page__inner">
                        <Pebbles label="signing you in" large=true/>
                        <h1 class="d-status-page__title">"signing you in"</h1>
                    </div>
                    <div class="d-status-page__foot">"pebblelab"</div>
                </main>
            }.into_any(),
        }}
    }
}

const SECTIONS: [(&str, &str, &str, &str, &str); 5] = [
    ("account", "home", "home", "your pebblelab account, and the apps that use it.", "layout-dashboard"),
    ("profile", "profile", "profile", "your name, picture and how to reach you, in every app.", ""),
    ("security", "sign-in and security", "security", "how you sign in, where you are signed in, and deleting your account.", ""),
    ("family", "family", "family", "optional. every app can use it to share between the people you choose.", ""),
    ("connectors", "connectors", "connectors", "give claude, chatgpt or any mcp client its own token, with its own permissions.", ""),
];

/// The signed-in frame: sidebar, app bar and the page. Sends you to sign in without a session.
#[component]
fn HubShell() -> impl IntoView {
    let app = AppState::new();
    provide_context(app);
    let nav = use_navigate();
    if api::load_token().is_none() {
        nav("/signin", Default::default());
    } else {
        let s = app;
        leptos::task::spawn_local(async move {
            match api::me().await {
                Ok(m) => s.me.set(Some(m)),
                Err(e) => return s.fail(&e),
            }
            // fin's card on home counts them
            if let Ok(a) = api::accounts().await {
                s.accounts.set(a);
            }
        });
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
    let path = use_location().pathname;
    let page = Memo::new(move |_| path.get().trim_start_matches('/').split('/').next().unwrap_or("").to_string());
    let section = Memo::new(move |_| *SECTIONS.iter().find(|s| s.0 == page.get()).unwrap_or(&SECTIONS[0]));

    let sign_out = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/auth/signout", &serde_json::json!({})).await;
            api::clear_token();
            app.me.set(None);
        });
    });
    view! {
        <SkipLink/>
        <Shell state=shell height="100dvh">
            <Sidebar>
                <SidebarHead><Switcher here=Here::Account on_sign_out=sign_out/></SidebarHead>
                <SidebarContent>
                    <SidebarGroup label="you">
                        <SidebarItem href="/account" icon="layout-dashboard" label="home" current=Signal::derive(move || page.get() == "account")/>
                        <HubItem href="/profile" icon=USER label="profile" current=Signal::derive(move || page.get() == "profile")/>
                        <HubItem href="/security" icon=LOCK label="security" current=Signal::derive(move || page.get() == "security")/>
                    </SidebarGroup>
                    <SidebarGroup label="shared with your apps">
                        <HubItem href="/family" icon=USERS label="family" current=Signal::derive(move || page.get() == "family")/>
                        <HubItem href="/connectors" icon=PLUG label="connectors" current=Signal::derive(move || page.get() == "connectors")/>
                    </SidebarGroup>
                </SidebarContent>
            </Sidebar>
            <ShellMain>
                <AppBar>
                    <SidebarToggle/>
                    <AppBarSep/>
                    <Breadcrumbs>
                        <Crumb href="/account">"account"</Crumb>
                        <CrumbCurrent>{move || section.get().2}</CrumbCurrent>
                    </Breadcrumbs>
                    <AppBarSpacer/>
                </AppBar>
                <Main>
                    <div class="pl-panel" style="padding:8px 0">
                        <div style="display:flex;flex-direction:column;gap:4px">
                            <h1 class="heading-xl" style="margin:0;color:var(--text-strong)">{move || section.get().1}</h1>
                            <span style="color:var(--text-secondary)">{move || section.get().3}</span>
                        </div>
                        <Outlet/>
                    </div>
                </Main>
            </ShellMain>
        </Shell>
    }
}

/// A sidebar row for pages whose icon is not in the bundled set: the same markup as `SidebarItem`.
#[component]
fn HubItem(href: &'static str, icon: &'static str, label: &'static str, #[prop(into)] current: Signal<bool>) -> impl IntoView {
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

#[component]
fn Home() -> impl IntoView {
    let app = expect_context::<AppState>();
    let security = LocalResource::new(|| async { api::get::<Security>("/me/security").await.ok() });
    let tokens = LocalResource::new(|| async { api::get::<Vec<Connector>>("/connectors").await.map(|l| l.len()).unwrap_or(0) });
    let family = move || app.me.with(|m| m.as_ref().and_then(|m| m.family.clone()));
    view! {
        <div class="pl-grid">
            <a class="pl-app" href=fin_href() rel="external" style="grid-column:1 / -1">
                <span class="pl-mark" aria-hidden="true"><FinGlyph/></span>
                <span style="display:flex;flex-direction:column;gap:2px;min-width:0"><b style="color:var(--text-strong);font-weight:500">"fin"</b><span style="color:var(--text-secondary)">{move || { let n = app.accounts.with(|a| a.iter().filter(|a| !a.archived).count()); if n == 0 { "a household money tracker".to_string() } else if n == 1 { "1 account".to_string() } else { format!("{n} accounts") } }}</span></span>
                <span class="d-btn d-btn--secondary">"open"</span>
            </a>
            <section class="d-card" aria-labelledby="h-fam">
                <header class="d-card__head"><h2 class="d-card__title" id="h-fam">"household"</h2><span class="d-card__meta">{move || family().map(|f| f.name).unwrap_or_else(|| "just you".into())}</span></header>
                <div class="d-card__body"><div class="d-setrows">
                    <div class="d-setrow">
                        <span class="d-setrow__text">
                            {move || match family() {
                                Some(f) => view! { <b>{f.name.clone()}</b><small>{f.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(" and ")}</small> }.into_any(),
                                None => view! { <b>"just you"</b><small>"a family is optional: add one to share between people."</small> }.into_any(),
                            }}
                        </span>
                        <ButtonLink href="/family" variant=ButtonVariant::Secondary>"manage"</ButtonLink>
                    </div>
                </div></div>
            </section>
            <section class="d-card" aria-labelledby="h-sec">
                <header class="d-card__head"><h2 class="d-card__title" id="h-sec">"sign-in"</h2></header>
                <div class="d-card__body"><div class="d-setrows">
                    {move || security.get().flatten().map(|s| view! {
                        {s.methods.iter().map(|m| view! {
                            <div class="d-setrow"><span class="d-setrow__text"><b>{m.provider.clone()}</b><small>{m.email.clone()}</small></span><span class="d-status" data-state="on" data-severity="success">"connected"</span></div>
                        }).collect_view()}
                        <div class="d-setrow"><span class="d-setrow__text"><b>{format!("{} places signed in", s.sessions.len())}</b><small>"this browser and your apps"</small></span><ButtonLink href="/security" variant=ButtonVariant::Secondary>"review"</ButtonLink></div>
                    })}
                </div></div>
            </section>
            <section class="d-card" aria-labelledby="h-cn">
                <header class="d-card__head"><h2 class="d-card__title" id="h-cn">"connectors"</h2></header>
                <div class="d-card__body"><div class="d-setrows">
                    <div class="d-setrow"><span class="d-setrow__text"><b>{move || format!("{} tokens", tokens.get().unwrap_or(0))}</b><small>"claude and other mcp clients, each with its own permissions"</small></span><ButtonLink href="/connectors" variant=ButtonVariant::Secondary>"manage"</ButtonLink></div>
                </div></div>
            </section>
        </div>
    }
}

#[component]
fn Profile() -> impl IntoView {
    view! { <ProfileTab/> }
}

#[component]
fn Family() -> impl IntoView {
    view! { <FamilyTab/> }
}

#[component]
fn ConnectorsPage() -> impl IntoView {
    view! { <Connectors/> }
}

/// `2026-10-05 12:30:00` (utc) as `today, 12:30`, `yesterday` or `05 oct`.
fn ago(ts: &str) -> String {
    let day = ts.get(..10).unwrap_or(ts);
    if day == crate::fmt::today() {
        format!("today, {}", ts.get(11..16).unwrap_or(""))
    } else if day == crate::fmt::days_ago(1) {
        "yesterday".into()
    } else {
        crate::fmt::day(day)
    }
}

#[component]
fn Security() -> impl IntoView {
    let app = expect_context::<AppState>();
    let rev = RwSignal::new(0u32);
    let data = LocalResource::new(move || {
        rev.track();
        async { api::get::<Security>("/me/security").await }
    });
    let notice = api::hash_params();
    let err = RwSignal::new(notice.get("error").cloned());
    let linked = notice.get("linked").cloned();
    if let Some(p) = linked {
        app.ok(&format!("{p} connected"));
    }
    let providers = LocalResource::new(api::providers);
    let confirm = RwSignal::new(false);
    let (del_text, del_err) = (RwSignal::new(String::new()), RwSignal::new(None::<String>));
    let email = move || app.me.with(|m| m.as_ref().map(|m| m.user.email.clone()).unwrap_or_default());

    let connect = move |provider: String| {
        err.set(None);
        leptos::task::spawn_local(async move {
            #[derive(serde::Deserialize)]
            struct Url {
                url: String,
            }
            match api::post::<Url>(&format!("/me/identities/{provider}/link"), &serde_json::json!({})).await {
                Ok(u) => {
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().set_href(&u.url);
                    }
                }
                Err(e) => err.set(Some(e.message)),
            }
        });
    };
    let disconnect = move |provider: String| {
        err.set(None);
        leptos::task::spawn_local(async move {
            match api::send::<serde_json::Value>("DELETE", &format!("/me/identities/{provider}"), &serde_json::json!({})).await {
                Ok(_) => rev.update(|r| *r += 1),
                Err(e) if e.status == 400 || e.status == 404 => err.set(Some(e.message)),
                Err(e) => app.fail(&e),
            }
        });
    };
    let end = move |id: i64| {
        leptos::task::spawn_local(async move {
            match api::send::<serde_json::Value>("DELETE", &format!("/sessions/{id}"), &serde_json::json!({})).await {
                Ok(_) => rev.update(|r| *r += 1),
                Err(e) => app.fail(&e),
            }
        });
    };
    let out_all = move |_| {
        leptos::task::spawn_local(async move {
            let _ = api::post::<serde_json::Value>("/auth/signout-all", &serde_json::json!({})).await;
            api::clear_token();
            app.me.set(None);
        });
    };
    let delete = move |_| {
        del_err.set(None);
        let typed = del_text.get_untracked();
        leptos::task::spawn_local(async move {
            match api::send::<serde_json::Value>("DELETE", "/me", &serde_json::json!({ "email": typed })).await {
                Ok(_) => {
                    api::clear_token();
                    app.me.set(None);
                }
                Err(e) if e.status == 400 || e.status == 403 => del_err.set(Some(e.message)),
                Err(e) => app.fail(&e),
            }
        });
    };
    let locked = Signal::derive(move || del_text.with(|t| t.trim().to_lowercase()) != email().to_lowercase());

    view! {
        {move || err.get().map(|m| view! { <span class="d-error" role="alert">{format!("error: {m}")}</span> })}
        <div class="pl-grid">
            <section class="d-card" aria-labelledby="sc-m">
                <header class="d-card__head"><h2 class="d-card__title" id="sc-m">"ways to sign in"</h2></header>
                <div class="d-card__body">
                    {move || data.get().and_then(|r| r.ok()).map(|s| {
                        let have: Vec<String> = s.methods.iter().map(|m| m.provider.clone()).collect();
                        let many = have.len() > 1;
                        let all = providers.get().unwrap_or_default();
                        view! {
                            <div class="d-setrows">
                                {s.methods.iter().map(|m| {
                                    let p = m.provider.clone();
                                    view! {
                                        <div class="d-setrow"><span class="d-setrow__text"><b>{m.provider.clone()}</b><small>{m.email.clone()}</small></span>
                                            {many.then(|| view! { <button type="button" class="d-btn d-btn--ghost" on:click=move |_| disconnect(p.clone())>"disconnect"</button> })}
                                            {(!many).then(|| view! { <span class="d-status" data-state="on" data-severity="success">"connected"</span> })}
                                        </div>
                                    }
                                }).collect_view()}
                                {all.into_iter().filter(|p| !have.contains(p)).map(|p| {
                                    let q = p.clone();
                                    view! {
                                        <div class="d-setrow"><span class="d-setrow__text"><b>{p.clone()}</b><small>"not connected"</small></span>
                                            <button type="button" class="d-btn d-btn--secondary" on:click=move |_| connect(q.clone())>{format!("connect {p}")}</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                            <p style="margin:12px 0 0;color:var(--text-secondary);font-size:var(--font-size-sm)">"keep two connected, so losing one does not lock you out."</p>
                        }
                    })}
                </div>
            </section>
            <section class="d-card" aria-labelledby="sc-s">
                <header class="d-card__head"><h2 class="d-card__title" id="sc-s">"where you are signed in"</h2></header>
                <div class="d-card__body">
                    <div class="d-setrows">
                        {move || data.get().and_then(|r| r.ok()).map(|s| s.sessions.into_iter().map(|x| {
                            let id = x.id;
                            let label = match (x.current, x.app.as_str()) {
                                (true, _) => "this browser".to_string(),
                                (false, "fin") => "fin".to_string(),
                                (false, "account") => "another browser".to_string(),
                                _ => "a browser".to_string(),
                            };
                            let meta = format!("signed in {}, last used {}", ago(&x.created_at), x.last_used_at.as_deref().map(ago).unwrap_or_else(|| "never".into()));
                            view! {
                                <div class="d-setrow"><span class="d-setrow__text"><b>{label}</b><small>{meta}</small></span>
                                    {(!x.current).then(|| view! { <button type="button" class="d-btn d-btn--ghost" on:click=move |_| end(id)>"sign out"</button> })}
                                </div>
                            }
                        }).collect_view())}
                        <div class="d-setrow"><span class="d-setrow__text"><b>"sign out everywhere"</b><small>"every browser and app. tokens keep working."</small></span>
                            <button type="button" class="d-btn d-btn--secondary" on:click=out_all>"sign out all"</button></div>
                    </div>
                </div>
            </section>
            <section class="d-card" aria-labelledby="sc-d">
                <header class="d-card__head"><h2 class="d-card__title" id="sc-d">"delete your account"</h2></header>
                <div class="d-card__body" style="display:flex;flex-direction:column;gap:12px">
                    <p style="margin:0">"removes your sign-in, your data in every app, and your place in the family. joint accounts stay with the other owners. it cannot be undone."</p>
                    {move || if confirm.get() {
                        view! {
                            <TextField label="type your email to confirm" value=del_text error=del_err placeholder=email()/>
                            <div class="d-row" style="gap:8px">
                                <Button variant=ButtonVariant::Secondary on:click=move |_| { confirm.set(false); del_text.set(String::new()); del_err.set(None); }>"cancel"</Button>
                                <Button variant=ButtonVariant::Danger disabled=locked attr:aria-describedby="del-why" on:click=delete><Ico d=TRASH/>"delete my account"</Button>
                            </div>
                            {move || locked.get().then(|| view! { <Reason id="del-why">"type your email above to unlock this."</Reason> })}
                        }.into_any()
                    } else {
                        view! { <div><button type="button" class="d-btn d-btn--danger" on:click=move |_| confirm.set(true)><Ico d=TRASH/>"delete account"</button></div> }.into_any()
                    }}
                </div>
            </section>
        </div>
    }
}
