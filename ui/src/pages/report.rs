use dots_ui::prelude::*;
use leptos::prelude::*;
use pebblelab_api::*;

use crate::api;
use crate::fmt;
use crate::icons::*;
use crate::state::AppState;

/// `/report`: one month, start to end, against the month before. It opens on the last whole month.
#[component]
pub fn MonthReport() -> impl IntoView {
    let app = expect_context::<AppState>();
    let this_month = fmt::today()[..7].to_string();
    let month = RwSignal::new(fmt::shift_month(&this_month, -1));
    let data = LocalResource::new(move || {
        app.rev.track();
        let m = month.get();
        async move { api::get::<Report>(&format!("/report?month={m}")).await }
    });
    let shown = RwSignal::new(None::<Result<Report, api::ApiError>>);
    Effect::new(move |_| {
        if let Some(r) = data.get() {
            shown.set(Some(r));
        }
    });
    let is_now = {
        let this_month = this_month.clone();
        Memo::new(move |_| month.get() == this_month)
    };
    let print = move |_| {
        if let Some(w) = web_sys::window() {
            let _ = w.print();
        }
    };

    view! {
        <div class="rp">
            <div class="d-row" style="justify-content:space-between;gap:8px">
                <div style="display:flex;flex-direction:column;gap:4px;min-width:0">
                    <h1 class="heading-xl">{move || format!("{}{}", fmt::month_year(&month.get()), if is_now.get() { " so far" } else { "" })}</h1>
                    <span style="color:var(--text-secondary)">"the month in review, against the one before"</span>
                </div>
                <nav class="d-row rp-nav" aria-label="month" style="gap:4px;flex-wrap:nowrap">
                    <button type="button" class="d-btn d-btn--ghost d-btn--icon" aria-label="print or save as pdf" title="print or save as pdf" on:click=print><Ico d=PRINTER/></button>
                    <button type="button" class="d-btn d-btn--ghost d-btn--icon" aria-label="previous month" on:click=move |_| month.update(|m| *m = fmt::shift_month(m, -1))><Ico d=PREV/></button>
                    <button type="button" class="d-btn d-btn--ghost d-btn--icon" aria-label="next month" disabled=move || is_now.get() on:click=move |_| month.update(|m| *m = fmt::shift_month(m, 1))><Ico d=NEXT/></button>
                </nav>
            </div>
            {move || match shown.get() {
                None => view! { <EmptyLoading title="loading the report"/> }.into_any(),
                Some(Err(e)) => view! { <EmptyError title="could not load the report" hint=e.message/> }.into_any(),
                Some(Ok(r)) => view! { <Body r=r/> }.into_any(),
            }}
        </div>
    }
}

/// `₹2,000 more than oct`, `₹500 less than oct`, or `same as oct`.
fn versus(app: &AppState, now: i64, before: i64, last: &str) -> String {
    match now - before {
        0 => format!("same as {last}"),
        d if d > 0 => format!("{} more than {last}", app.money(d)),
        d => format!("{} less than {last}", app.money(-d)),
    }
}

#[component]
fn Body(r: Report) -> impl IntoView {
    let app = expect_context::<AppState>();
    let last = fmt::month(&fmt::shift_month(&r.month, -1));
    let kept = r.income - r.spending;
    let last_kept = r.last_income - r.last_spending;
    let figures = [
        ("came in", app.money(r.income), versus(&app, r.income, r.last_income, &last)),
        ("went out", app.money(r.spending), versus(&app, r.spending, r.last_spending, &last)),
        ("kept", app.signed(kept), format!("{} of income · {}", fmt::pct(kept.max(0), r.income), versus(&app, kept, last_kept, &last))),
    ];
    let net = match (&r.net_start, &r.net_end) {
        (Some(a), Some(b)) => {
            let (from, to) = (a.own - a.owe, b.own - b.owe);
            let moved = if to >= from { format!("up {}", app.money(to - from)) } else { format!("down {}", app.money(from - to)) };
            Some((app.signed(to), format!("{moved} from {} at the end of {last}", app.signed(from))))
        }
        _ => None,
    };
    let empty = r.income == 0 && r.spending == 0 && r.last_income == 0 && r.last_spending == 0;
    let top = r.categories.iter().map(|c| c.total.max(c.last_month)).max().unwrap_or(0).max(1) as f64;

    view! {
        <section class="d-card" aria-label="the month in figures">
            <div class="d-card__body rp-figs">
                {figures.into_iter().map(|(label, value, note)| view! {
                    <div class="rp-fig">
                        <span class="rp-fig__label">{label}</span>
                        <span class="heading-2xl rp-fig__value">{value}</span>
                        <span class="rp-fig__note">{note}</span>
                    </div>
                }).collect_view()}
                {net.map(|(value, note)| view! {
                    <div class="rp-fig">
                        <span class="rp-fig__label">"net worth"</span>
                        <span class="heading-2xl rp-fig__value">{value}</span>
                        <span class="rp-fig__note">{note}</span>
                    </div>
                })}
            </div>
        </section>
        {if empty {
            view! { <EmptyState title="nothing this month" hint="money in and out with a date in this month shows here, against the month before."/> }.into_any()
        } else {
            view! {
                <section class="d-card" aria-labelledby="rp-cat">
                    <header class="d-card__head"><h2 class="d-card__title" id="rp-cat">"where it went"</h2><span class="d-card__meta">{format!("by tag · {last} in grey")}</span></header>
                    <div class="d-card__body">
                        <ul class="rp-cats">
                            {r.categories.iter().map(|c| {
                                let (w, lw) = (c.total as f64 / top * 100.0, c.last_month as f64 / top * 100.0);
                                let change = if c.last_month == 0 { "new".to_string() } else if c.total == 0 { "none this month".to_string() } else { versus(&app, c.total, c.last_month, &last) };
                                view! {
                                    <li class="rp-cat">
                                        <span class="d-badge" data-cat=tag_cat(&c.tag).to_string()>{c.tag.clone()}</span>
                                        <span class="rp-cat__bars" role="img" aria-label=format!("{}: {}, {} {}", c.tag, app.money(c.total), last, app.money(c.last_month))>
                                            <span class="rp-bar" style=format!("width:{w:.1}%")></span>
                                            <span class="rp-bar rp-bar--before" style=format!("width:{lw:.1}%")></span>
                                        </span>
                                        <span class="rp-cat__fig">{app.money(c.total)}</span>
                                        <span class="rp-cat__change" data-up=(c.total > c.last_month && c.last_month > 0).then_some("")>{change}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </div>
                </section>
                <div class="rp-pair">
                    <section class="d-card" aria-labelledby="rp-big">
                        <header class="d-card__head"><h2 class="d-card__title" id="rp-big">"biggest spends"</h2></header>
                        <div class="d-card__body">
                            {if r.biggest.is_empty() {
                                view! { <span style="color:var(--text-secondary)">"no money went out this month"</span> }.into_any()
                            } else {
                                view! {
                                    <ol class="rp-list">
                                        {r.biggest.iter().map(|b| view! {
                                            <li class="rp-row">
                                                <span class="rp-row__main">
                                                    <span class="rp-row__title">{if b.description.is_empty() { b.tag.clone() } else { b.description.clone() }}</span>
                                                    <small>{format!("{} · {} · {}", fmt::day(&b.date), b.account, b.tag)}</small>
                                                </span>
                                                <span class="rp-row__fig">{app.money(b.amount)}</span>
                                            </li>
                                        }).collect_view()}
                                    </ol>
                                }.into_any()
                            }}
                        </div>
                    </section>
                    <section class="d-card" aria-labelledby="rp-bud">
                        <header class="d-card__head"><h2 class="d-card__title" id="rp-bud">"budgets"</h2><a class="d-card__meta" href="/budgets">"change"</a></header>
                        <div class="d-card__body">
                            {if r.budgets.is_empty() {
                                view! { <span style="color:var(--text-secondary)">"no budgets set. "<a href="/budgets">"set one"</a>" to see how each month holds up."</span> }.into_any()
                            } else {
                                view! {
                                    <ul class="rp-list">
                                        {r.budgets.iter().map(|b| {
                                            let over = b.spent > b.amount;
                                            let sev = if over { Some(Severity::Danger) } else if b.spent * 100 >= b.amount * 80 { Some(Severity::Warning) } else { None };
                                            let pct = (b.spent.max(0) as f64 * 100.0 / b.amount.max(1) as f64).min(100.0);
                                            let label = format!("{} of {}", app.money(b.spent.max(0)), app.money(b.amount));
                                            view! {
                                                <li class="rp-bud">
                                                    <span class="rp-row">
                                                        <span class="d-badge" data-cat=tag_cat(&b.tag).to_string()>{b.tag.clone()}</span>
                                                        <span class="rp-row__fig" data-over=over.then_some("")>{if over { format!("{} over", app.money(b.spent - b.amount)) } else { label.clone() }}</span>
                                                    </span>
                                                    <Meter value=Signal::stored(pct) label=format!("{} budget", b.tag) text=label severity=sev/>
                                                </li>
                                            }
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }}
                        </div>
                    </section>
                </div>
            }.into_any()
        }}
    }
}
