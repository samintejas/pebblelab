use dots_ui::prelude::*;
use leptos::prelude::*;
use pebblelab_api::money::{format_minor, parse_minor};
use pebblelab_api::*;

use crate::api;
use crate::fmt;
use crate::icons::*;
use crate::state::AppState;

/// How full a budget is, and how that reads: nearly there at 80%, over past 100%.
fn severity(b: &Budget) -> Option<Severity> {
    match b.spent * 100 / b.amount.max(1) {
        p if p > 100 => Some(Severity::Danger),
        p if p >= 80 => Some(Severity::Warning),
        _ => None,
    }
}

/// `/budgets`: a monthly limit on a tag, and what the month has used of it.
#[component]
pub fn Budgets() -> impl IntoView {
    let app = expect_context::<AppState>();
    let this_month = fmt::today()[..7].to_string();
    let month = RwSignal::new(this_month.clone());
    let data = LocalResource::new(move || {
        app.rev.track();
        let m = month.get();
        async move { api::get::<Vec<Budget>>(&format!("/budgets?month={m}")).await }
    });
    let shown = RwSignal::new(None::<Result<Vec<Budget>, api::ApiError>>);
    Effect::new(move |_| {
        if let Some(r) = data.get() {
            shown.set(Some(r));
        }
    });

    // the form: a tag and its limit; a tag that has one already gets the new amount
    let tag = RwSignal::new(String::new());
    let amount = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let set = move || {
        let t = tag.get_untracked().trim().to_lowercase();
        let a = match parse_minor(amount.get_untracked().trim()) {
            Ok(a) if a > 0 && !t.is_empty() => a,
            _ => return err.set(Some("choose a tag and a monthly amount above zero".into())),
        };
        err.set(None);
        busy.set(true);
        leptos::task::spawn_local(async move {
            let r = api::post::<Budget>("/budgets", &serde_json::json!({"tag": t, "amount": format_minor(a)})).await;
            busy.set(false);
            match r {
                Ok(b) => {
                    tag.set(String::new());
                    amount.set(String::new());
                    app.ok(&format!("{} budget set", b.tag));
                    app.bump();
                }
                Err(e) if e.status == 400 || e.status == 403 => err.set(Some(e.message)),
                Err(e) => app.fail(&e),
            }
        });
    };
    let remove = move |id: i64| {
        leptos::task::spawn_local(async move {
            match api::delete(&format!("/budgets/{id}")).await {
                Ok(_) => app.bump(),
                Err(e) => app.fail(&e),
            }
        });
    };
    let edit = move |b: &Budget| {
        tag.set(b.tag.clone());
        amount.set(fmt::plain(b.amount));
    };

    let is_now = {
        let this_month = this_month.clone();
        Memo::new(move |_| month.get() == this_month)
    };
    let summary = move || match shown.get() {
        Some(Ok(list)) if !list.is_empty() => {
            let (spent, limit): (i64, i64) = list.iter().fold((0, 0), |(s, l), b| (s + b.spent.max(0), l + b.amount));
            let over = list.iter().filter(|b| b.spent > b.amount).count();
            format!("{} of {} used{}", app.money(spent), app.money(limit), if over > 0 { format!(" · {over} over") } else { String::new() })
        }
        _ => "a monthly limit on a tag. money out with the tag counts, refunds take it back, transfers never count".to_string(),
    };

    view! {
        <div style="display:flex;flex-direction:column;gap:16px;max-width:880px">
            <div style="display:flex;flex-direction:column;gap:4px">
                <h1 class="heading-xl">"budgets"</h1>
                <span style="color:var(--text-secondary)">{summary}</span>
            </div>
            <form class="d-card" on:submit=move |e| { e.prevent_default(); set() }>
                <div class="d-card__body" style="display:flex;flex-wrap:wrap;gap:12px;align-items:flex-end">
                    <div class="d-field" style="flex:2 1 200px">
                        <label class="d-label" for="bg-tag">"tag"</label>
                        <input class="d-input" id="bg-tag" list="bg-tags" placeholder="groceries" autocomplete="off"
                            prop:value=move || tag.get() on:input=move |e| tag.set(event_target_value(&e))/>
                        <datalist id="bg-tags">{move || app.tags.get().into_iter().map(|t| view! { <option value=t/> }).collect_view()}</datalist>
                    </div>
                    <div class="d-field" style="flex:1 1 140px">
                        <label class="d-label" for="bg-amt">{move || format!("every month ({})", fmt::symbol(&app.currency()))}</label>
                        <input class="d-input" id="bg-amt" type="text" inputmode="decimal" placeholder="12000"
                            prop:value=move || amount.get() on:input=move |e| amount.set(event_target_value(&e))/>
                    </div>
                    <button type="submit" class="d-btn d-btn--primary" disabled=move || busy.get()><Ico d=CHECK/>"set budget"</button>
                    {move || err.get().map(|e| view! { <span class="d-hint" style="flex-basis:100%;color:var(--accent-danger)">{e}</span> })}
                </div>
            </form>
            <div class="d-row" style="justify-content:space-between;gap:8px">
                <h2 class="heading-lg">{move || if is_now.get() { format!("{} so far", fmt::month_year(&month.get())) } else { fmt::month_year(&month.get()) }}</h2>
                <nav class="d-row" aria-label="month" style="gap:4px;flex-wrap:nowrap">
                    <button type="button" class="d-btn d-btn--ghost d-btn--icon" aria-label="previous month" on:click=move |_| month.update(|m| *m = fmt::shift_month(m, -1))><Ico d=PREV/></button>
                    <button type="button" class="d-btn d-btn--ghost d-btn--icon" aria-label="next month" disabled=move || is_now.get() on:click=move |_| month.update(|m| *m = fmt::shift_month(m, 1))><Ico d=NEXT/></button>
                </nav>
            </div>
            {move || match shown.get() {
                None => view! { <EmptyLoading title="loading budgets"/> }.into_any(),
                Some(Err(e)) => view! { <EmptyError title="could not load budgets" hint=e.message/> }.into_any(),
                Some(Ok(list)) if list.is_empty() => view! {
                    <EmptyState title="no budgets yet" hint="pick a tag you spend on, like groceries or dining, and how much a month is enough. you see how far each month gets."/>
                }.into_any(),
                Some(Ok(list)) => view! {
                    <ul class="bg-list" aria-label="budgets">
                        {list.into_iter().map(|b| {
                            let id = b.id;
                            let sev = severity(&b);
                            let pct = (b.spent.max(0) as f64 * 100.0 / b.amount.max(1) as f64).min(100.0);
                            let left = b.amount - b.spent;
                            let status = if left >= 0 { format!("{} left", app.money(left)) } else { format!("{} over", app.money(-left)) };
                            let before = format!("{} last month", app.money(b.last_month.max(0)));
                            let label = format!("{} of {}", app.money(b.spent.max(0)), app.money(b.amount));
                            let b2 = b.clone();
                            view! {
                                <li class="bg-item">
                                    <div class="bg-item__top">
                                        <span class="d-badge" data-cat=tag_cat(&b.tag).to_string()>{b.tag.clone()}</span>
                                        <span class="bg-item__fig">{label.clone()}</span>
                                    </div>
                                    <Meter value=Signal::stored(pct) label=format!("{} budget", b.tag) text=label severity=sev/>
                                    <div class="bg-item__foot">
                                        <span><span data-over=(left < 0).then_some("")>{status}</span>" · "{before}</span>
                                        <span class="bg-item__acts">
                                            <button type="button" class="d-btn d-btn--ghost d-btn--sm" on:click=move |_| edit(&b2)>"change"</button>
                                            <button type="button" class="d-btn d-btn--ghost d-btn--icon d-btn--sm" aria-label=format!("delete the {} budget", b.tag) on:click=move |_| remove(id)><Ico d=TRASH/></button>
                                        </span>
                                    </div>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </div>
    }
}
