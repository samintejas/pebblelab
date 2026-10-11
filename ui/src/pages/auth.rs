use dots_ui::prelude::*;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use pebblelab_api::*;

use crate::api;
use crate::icons::Logo;
use crate::site::{self, use_site};

/// On an address that only serves an app, these pages live in the account: go there, keeping where the
/// person was headed. True when the browser is leaving.
fn send_to_account(path: &str) -> bool {
    let site = use_site();
    let go = move || {
        if site.is_fin_only() {
            let hash = web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default();
            site::go_external(&format!("{}{path}?app=fin{hash}", site.get().account_url));
            true
        } else {
            false
        }
    };
    if site.ready() {
        return go();
    }
    let path = path.to_string();
    Effect::new(move |_| {
        if site.ready() && site.is_fin_only() {
            let hash = web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default();
            site::go_external(&format!("{}{path}?app=fin{hash}", site.get().account_url));
        }
    });
    false
}

/// "to continue to fin" under a sign-in title, when an app sent the person here.
fn continuing() -> Option<String> {
    site::requested_app().map(|a| format!("to continue to {a}."))
}

/// Signed in: go on to the app that asked, or the account.
fn finish(site: site::SiteCtx, nav: impl Fn(&str, leptos_router::NavigateOptions) + Clone + 'static, err: RwSignal<Option<String>>) {
    leptos::task::spawn_local(async move {
        match site::continue_after_sign_in(&site, site::requested_app()).await {
            Ok(path) if !path.is_empty() => nav(&path, Default::default()),
            Ok(_) => {}
            Err(e) => err.set(Some(e.message)),
        }
    });
}

#[component]
fn AuthPage(title: &'static str, #[prop(into)] sub: String, other: AnyView, #[prop(default = 380)] width: u32, children: Children) -> impl IntoView {
    view! {
        <SkipLink/>
        <div style="min-height:100dvh;display:flex;flex-direction:column;background:var(--bg-base)">
            <AppBar>
                <A href="/" attr:aria-label="pebblelab, home" attr:style="display:flex;margin-left:4px;flex:none"><Logo height=18/></A>
                <AppBarSpacer/>
                {other}
            </AppBar>
            <main id="main" tabindex="-1" style="flex:1;display:flex;align-items:center;justify-content:center;padding:48px 16px">
                <div class="d-card" style=format!("width:100%;max-width:{width}px")>
                    <div class="d-card__body" style="padding:24px;display:flex;flex-direction:column;gap:16px">
                        <div style="display:flex;flex-direction:column;gap:4px">
                            <h1 class="heading-xl">{title}</h1>
                            <p style="color:var(--text-secondary)">{sub}</p>
                        </div>
                        {children()}
                    </div>
                </div>
            </main>
        </div>
    }
}

fn other(prompt: &'static str, href: &'static str, label: &'static str) -> AnyView {
    view! {
        <span style="color:var(--text-secondary)">{prompt}</span>
        <ButtonLink href=href variant=ButtonVariant::Secondary size=Size::Sm>{label}</ButtonLink>
    }
    .into_any()
}

/// "or continue with google / github", for the providers this server has set up. Nothing shows when none are.
#[component]
fn Providers(verb: &'static str, #[prop(optional)] alone: bool) -> impl IntoView {
    let list = LocalResource::new(api::providers);
    view! {
        {move || list.get().filter(|l| !l.is_empty()).map(|l| view! {
            <div style="display:flex;flex-direction:column;gap:8px">
                {(!alone).then(|| view! {
                    <div class="d-row" style="gap:12px;color:var(--text-secondary)" aria-hidden="true">
                        <span style="flex:1;border-top:1px solid var(--border-default)"></span>"or"<span style="flex:1;border-top:1px solid var(--border-default)"></span>
                    </div>
                })}
                {l.into_iter().map(|p| view! {
                    // a full page load: the provider is another site, so the router must leave it alone
                    <a class="d-btn d-btn--secondary d-btn--lg" rel="external" href=format!("/api/auth/{p}/start") style="width:100%;justify-content:center"
                        on:click=|_| site::remember_app(site::requested_app().as_deref())>
                        {format!("{verb} with {p}")}
                    </a>
                }).collect_view()}
            </div>
        })}
    }
}

/// Where a provider sends the person after they sign in: swap the one-time code in the address for a session.
#[component]
pub fn AuthCallback() -> impl IntoView {
    let err = RwSignal::new(None::<String>);
    let nav = use_navigate();
    let site = use_site();
    // the code is read once, before the address changes
    let code = api::hash_params().remove("code");
    let started = StoredValue::new(false);
    Effect::new(move |_| {
        if !site.ready() || started.get_value() {
            return;
        }
        started.set_value(true);
        let (nav, code) = (nav.clone(), code.clone());
        leptos::task::spawn_local(async move {
            let Some(code) = code else {
                return err.set(Some("there is no sign-in to finish: start again".into()));
            };
            match api::post::<Session>("/auth/redeem", &serde_json::json!({ "code": code })).await {
                Ok(s) => {
                    api::save_token(&s.token, true);
                    // handed over by the account: this is the app, open it
                    if site.is_fin_only() {
                        return nav("/transactions", Default::default());
                    }
                    // back from google or github: on to wherever the sign-in started from
                    match site::continue_after_sign_in(&site, None).await {
                        Ok(path) if !path.is_empty() => nav(&path, Default::default()),
                        Ok(_) => {}
                        Err(e) => err.set(Some(e.message)),
                    }
                }
                Err(e) => err.set(Some(if e.unauthorized() { "this sign-in expired or was already used: start again".into() } else { e.message })),
            }
        });
    });
    view! {
        <AuthPage title="signing you in" sub="one moment." other=other("", "/signin", "sign in")>
            {move || err.get().map(|m| view! {
                <div role="alert" style="display:flex;flex-direction:column;gap:12px">
                    <span class="d-error">{format!("error: {m}")}</span>
                    <A href="/signin" attr:class="d-link" attr:style="align-self:flex-start">"back to sign in"</A>
                </div>
            })}
        </AuthPage>
    }
}

#[component]
pub fn SignIn() -> impl IntoView {
    let (email, pass, keep) = (RwSignal::new(String::new()), RwSignal::new(String::new()), RwSignal::new(true));
    // a provider sign-in that failed lands here with the reason after the `#`
    let (busy, err) = (RwSignal::new(false), RwSignal::new(api::hash_params().remove("error")));
    let pw = LocalResource::new(api::password_login);
    let nav = use_navigate();
    let leaving = send_to_account("/signin");
    let site = use_site();
    // already signed in to the account: go straight on
    {
        let (site, nav, started) = (use_site(), nav.clone(), StoredValue::new(false));
        Effect::new(move |_| {
            if leaving || !site.ready() || site.is_fin_only() || started.get_value() || api::load_token().is_none() {
                return;
            }
            started.set_value(true);
            let nav = nav.clone();
            leptos::task::spawn_local(async move {
                match site::continue_after_sign_in(&site, site::requested_app()).await {
                    Ok(path) if !path.is_empty() => nav(&path, Default::default()),
                    Ok(_) => {}
                    // the stored session has ended: sign in again
                    Err(_) => api::clear_token(),
                }
            });
        });
    }
    let submit = Callback::new(move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        busy.set(true);
        err.set(None);
        let nav = nav.clone();
        leptos::task::spawn_local(async move {
            match api::post::<Session>("/auth/signin", &serde_json::json!({"email": email.get_untracked(), "password": pass.get_untracked()})).await {
                Ok(s) => {
                    api::save_token(&s.token, keep.get_untracked());
                    finish(site, nav, err);
                }
                Err(e) => {
                    err.set(Some(if e.unauthorized() { "email or password is wrong".into() } else { e.message }));
                    busy.set(false);
                }
            }
        });
    });
    view! {
        <AuthPage title="sign in" sub=continuing().unwrap_or_else(|| "to your pebblelab account.".into()) other=other("new here?", "/signup", "create an account")>
            {move || pw.get().map(|on| if on {
                view! {
                    <form on:submit=move |e| submit.run(e) style="display:flex;flex-direction:column;gap:16px">
                        <TextField label="email" value=email input_type="email" size=Size::Lg autocomplete="email" error=err/>
                        <TextField label="password" value=pass input_type="password" size=Size::Lg autocomplete="current-password"/>
                        <div class="d-row" style="justify-content:space-between">
                            <Checkbox checked=keep>"keep me signed in"</Checkbox>
                            <A href="/reset" attr:class="d-link">"forgot password"</A>
                        </div>
                        <Button variant=ButtonVariant::Primary size=Size::Lg submit=true busy=busy attr:style="width:100%">"sign in"</Button>
                    </form>
                    <Providers verb="sign in"/>
                }.into_any()
            } else {
                view! {
                    {move || err.get().map(|m| view! { <span class="d-error" role="alert">{format!("error: {m}")}</span> })}
                    <Providers verb="sign in" alone=true/>
                }.into_any()
            })}
        </AuthPage>
    }
}

#[component]
pub fn SignUp() -> impl IntoView {
    let (name, email, pass) = (RwSignal::new(String::new()), RwSignal::new(String::new()), RwSignal::new(String::new()));
    let (busy, err) = (RwSignal::new(false), RwSignal::new(None::<String>));
    let pw = LocalResource::new(api::password_login);
    let nav = use_navigate();
    send_to_account("/signup");
    let site = use_site();
    let submit = Callback::new(move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        busy.set(true);
        err.set(None);
        let nav = nav.clone();
        leptos::task::spawn_local(async move {
            match api::post::<Session>("/auth/signup", &serde_json::json!({"name": name.get_untracked(), "email": email.get_untracked(), "password": pass.get_untracked()})).await {
                Ok(s) => {
                    api::save_token(&s.token, true);
                    finish(site, nav, err);
                }
                Err(e) => {
                    err.set(Some(e.message));
                    busy.set(false);
                }
            }
        });
    });
    view! {
        <AuthPage title="create a pebblelab account" sub="one account for every pebblelab app. a family is optional and can be added later." width=420 other=other("have an account?", "/signin", "sign in")>
            {move || pw.get().map(|on| if on {
                view! {
                    <form on:submit=move |e| submit.run(e) style="display:flex;flex-direction:column;gap:16px">
                        <TextField label="name" value=name size=Size::Lg autocomplete="name"/>
                        <TextField label="email" value=email input_type="email" size=Size::Lg autocomplete="email"/>
                        <TextField label="password" value=pass input_type="password" size=Size::Lg autocomplete="new-password" hint="12 characters or more" error=err/>
                        <Button variant=ButtonVariant::Primary size=Size::Lg submit=true busy=busy attr:style="width:100%">"create account"</Button>
                    </form>
                    <Providers verb="sign up"/>
                }.into_any()
            } else {
                view! { <Providers verb="sign up" alone=true/> }.into_any()
            })}
        </AuthPage>
    }
}

#[component]
pub fn Reset() -> impl IntoView {
    send_to_account("/reset");
    let token = api::hash_params().remove("token");
    let only_sign_in = view! { <ButtonLink href="/signin" variant=ButtonVariant::Secondary size=Size::Sm>"sign in"</ButtonLink> }.into_any();
    let (email, pass) = (RwSignal::new(String::new()), RwSignal::new(String::new()));
    let (busy, err, done) = (RwSignal::new(false), RwSignal::new(None::<String>), RwSignal::new(false));
    let has_token = token.is_some();
    let pw = LocalResource::new(api::password_login);
    let submit = Callback::new(move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        busy.set(true);
        err.set(None);
        let token = token.clone();
        leptos::task::spawn_local(async move {
            let res = match token {
                Some(t) => api::post::<serde_json::Value>("/auth/reset/confirm", &serde_json::json!({ "token": t, "password": pass.get_untracked() })).await,
                None => api::post::<serde_json::Value>("/auth/reset", &serde_json::json!({ "email": email.get_untracked() })).await,
            };
            match res {
                Ok(_) => done.set(true),
                Err(e) => err.set(Some(e.message)),
            }
            busy.set(false);
        });
    });
    let sub = if has_token { "choose a new password. you will be signed out everywhere else." } else { "we will email you a link to choose a new one." };
    view! {
        <AuthPage title="reset password" sub=sub other=only_sign_in>
            {move || (pw.get() == Some(false)).then(|| view! {
                <div role="status" style="display:flex;flex-direction:column;gap:16px">
                    <p style="margin:0;max-width:64ch">"there is no password here: sign in with google or github."</p>
                    <A href="/signin" attr:class="d-link" attr:style="align-self:flex-start">"back to sign in"</A>
                </div>
            })}
            {move || if pw.get() != Some(true) {
                ().into_any()
            } else if done.get() {
                view! {
                    <div role="status" style="display:flex;flex-direction:column;gap:16px">
                        <p style="margin:0;max-width:64ch">{if has_token { "your password is changed. sign in with the new one." } else { "if that address has an account, a link is on its way. it works once, for an hour." }}</p>
                        <A href="/signin" attr:class="d-link" attr:style="align-self:flex-start">"back to sign in"</A>
                    </div>
                }.into_any()
            } else {
                view! {
                    <form on:submit=move |e| submit.run(e) style="display:flex;flex-direction:column;gap:16px">
                        {if has_token {
                            view! { <TextField label="new password" value=pass input_type="password" size=Size::Lg autocomplete="new-password" hint="12 characters or more" error=err/> }.into_any()
                        } else {
                            view! { <TextField label="email" value=email input_type="email" size=Size::Lg autocomplete="email" error=err/> }.into_any()
                        }}
                        <Button variant=ButtonVariant::Primary size=Size::Lg submit=true busy=busy attr:style="width:100%">{if has_token { "set password" } else { "send link" }}</Button>
                    </form>
                }.into_any()
            }}
        </AuthPage>
    }
}
