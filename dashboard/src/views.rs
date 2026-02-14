use std::sync::Arc;

use maud::{Markup, PreEscaped, html};
use rocket::{State, get};

use crate::{
    state::{AppState, ONLINE_GRACE_PERIOD},
    views::breadcrumbs::{Breadcrumb, breadcrumbs},
};

mod breadcrumbs;

turf::style_sheet!("assets/main.css");

fn page_shell(title: &str, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                style { (PreEscaped(STYLE_SHEET)) }
                script
                    src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.8/dist/htmx.min.js"
                    integrity="sha384-/TgkGk7p307TH7EXJDuUlgG3Ce1UVolAOFopFekQkkXihi5u/6OCvVKyz1W+idaz"
                    crossorigin="anonymous" {}
            }
            body {
                (content)
            }
        }
    }
}

fn format_relative(seconds_ago: i64) -> String {
    if seconds_ago < 60 {
        format!("{seconds_ago}s ago")
    } else if seconds_ago < 3600 {
        format!("{}m ago", seconds_ago / 60)
    } else if seconds_ago < 86400 {
        format!("{}h ago", seconds_ago / 3600)
    } else {
        format!("{}d ago", seconds_ago / 86400)
    }
}

#[get("/")]
pub fn index(state: &State<Arc<AppState>>) -> Markup {
    let content = html! {
        (breadcrumbs(&[Breadcrumb { label: "bots", href: None}]))
        div
            hx-get="/fragments/bot-list"
            hx-trigger="every 30s"
            hx-swap="innerHTML"
        {
            (bot_list_inner(state))
        }
    };
    page_shell("Dashboard | discord-bots", content)
}

fn bot_list_inner(state: &State<Arc<AppState>>) -> Markup {
    let registry = state.registry.read().unwrap();
    let mut bots = registry.bots();
    bots.sort_by_key(|b| b.name.clone());

    html! {
        @if bots.is_empty() {
            p.empty { "No bots registered. Send a heartbeat to get started." }
        } @else {
            div.bot-grid {
                @for bot in &bots {
                    @let online = registry.is_online(&bot.name, ONLINE_GRACE_PERIOD);
                    @let ago = (chrono::Utc::now() - bot.last_heartbeat).num_seconds();
                    a href=(format!("/bot/{}", bot.name)) {
                        div.bot-card {
                            div.bot-name { (bot.name) }
                            @if online {
                                div.status.online { "[ONLINE]" }
                            } @else {
                                div.status.offline { "[OFFLINE]" }
                            }
                            div.meta { "Last seen: " (format_relative(ago)) }
                        }
                    }
                }
            }
        }
    }
}

#[get("/fragments/bot-list")]
pub fn fragment_bot_list(state: &State<Arc<AppState>>) -> Markup {
    bot_list_inner(state)
}

#[get("/bot/<name>")]
pub fn bot_detail(name: &str, state: &State<Arc<AppState>>) -> Option<Markup> {
    let registry = state.registry.read().unwrap();
    let bot = registry.get(name)?;
    let online = registry.is_online(name, ONLINE_GRACE_PERIOD);
    let ago = (chrono::Utc::now() - bot.last_heartbeat).num_seconds();
    let bot_name = bot.name.clone();
    drop(registry);

    let content = html! {
        (breadcrumbs(&[
            Breadcrumb { label: "bots", href: Some("/")},
            Breadcrumb { label: name, href: None }])
        )

        @if online {
            div.status.online { "[ONLINE]" }
        } @else {
            div.status.offline { "[OFFLINE]" }
        }
        div.meta { "Last seen: " (format_relative(ago)) }

        div
            hx-get=(format!("/fragments/bot/{bot_name}/charts"))
            hx-trigger="every 60s"
            hx-swap="innerHTML"
        {
            (fragment_bot_charts(name, state)?)
        }
    };
    Some(page_shell(&format!("{bot_name} | Dashboard"), content))
}

fn charts_inner(uptime_svg: &str, metric_charts: &[(String, &str)]) -> Markup {
    html! {
        h2 { "> uptime" }
        div.chart-container {
            (PreEscaped(uptime_svg))
        }

        @if !metric_charts.is_empty() {
            h2 { "> metrics" }
            @for (_event_id, svg) in metric_charts {
                div.chart-container {
                    (PreEscaped(svg))
                }
            }
        }
    }
}

#[get("/fragments/bot/<name>/charts")]
pub fn fragment_bot_charts(name: &str, state: &State<Arc<AppState>>) -> Option<Markup> {
    let uptime_svg = "TODO uptime chart";

    let metrics = state.metrics.read().unwrap();
    let event_ids = metrics.event_ids(name);
    let mut metric_charts = Vec::new();
    for eid in &event_ids {
        let events = metrics.query(name, Some(eid));
        let has_values = events.iter().any(|e| e.value.is_some());
        let svg = if has_values {
            "TODO value chart"
        } else {
            "TODO count chart"
        };
        metric_charts.push((eid.clone(), svg));
    }
    drop(metrics);

    Some(charts_inner(uptime_svg, &metric_charts))
}
