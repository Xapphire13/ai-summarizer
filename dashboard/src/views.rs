use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use maud::{Markup, html};

use crate::state::AppState;
use crate::views::breadcrumbs::{Breadcrumb, breadcrumbs};

pub mod bot_detail;
pub mod bot_list;
mod breadcrumbs;
pub mod chart_actions;

fn page_shell(title: &str, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                link rel="stylesheet" href="/styles.css";
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

pub async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        crate::styles::ALL.clone(),
    )
}

pub async fn index(State(state): State<Arc<AppState>>) -> Markup {
    let content = html! {
        (breadcrumbs(&[Breadcrumb { label: "bots", href: None}]))
        div
            hx-get="/fragments/bot-list"
            hx-trigger="every 30s"
            hx-swap="innerHTML"
        {
            (bot_list::bot_list_inner(&state))
        }
    };
    page_shell("Dashboard | discord-bots", content)
}
