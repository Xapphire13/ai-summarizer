use std::collections::VecDeque;
use std::sync::Arc;

use axum::extract::State;
use chrono::{DateTime, Utc};
use maud::{Markup, PreEscaped, html};

use crate::state::{AppState, ONLINE_GRACE_PERIOD};
use crate::styles::BotList as ClassName;

fn uptime_blocks(history: &VecDeque<DateTime<Utc>>) -> [bool; 12] {
    let now = Utc::now();
    let mut blocks = [false; 12];
    for ts in history {
        let hours_ago = (now - *ts).num_hours();
        if (0..12).contains(&hours_ago) {
            blocks[11 - hours_ago as usize] = true;
        }
    }
    blocks
}

pub fn bot_list_inner(state: &Arc<AppState>) -> Markup {
    let registry = state.registry.read().unwrap();
    let mut bots = registry.bots();
    bots.sort_by_key(|b| b.name.clone());

    html! {
        @if bots.is_empty() {
            p { "No bots registered. Send a heartbeat to get started." }
        } @else {
            div.(ClassName::MENU) {
                @for bot in &bots {
                    @let online = registry.is_online(&bot.name, ONLINE_GRACE_PERIOD);
                    @let blocks = uptime_blocks(&bot.heartbeat_history);
                    a.(ClassName::MENU_ROW) href=(format!("/bot/{}", bot.name)) {
                        span.(ClassName::CHECKBOX) { "[ ]" }
                        span.(ClassName::CHECKBOX_ON) { "[x]" }
                        span.(ClassName::BOT_NAME) { (bot.name) }
                        span.(ClassName::UPTIME_BAR) {
                            @for &filled in &blocks {
                                @if filled {
                                    span.(ClassName::BLOCK_FILLED) { "▮" }
                                } @else {
                                    span.(ClassName::BLOCK_EMPTY) { "▯" }
                                }
                            }
                        }
                        @if online {
                            span.(ClassName::STATUS_ONLINE) { (PreEscaped("&nbsp;")) "[ONLINE]" }
                        } @else {
                            span.(ClassName::STATUS_OFFLINE) { "[OFFLINE]" }
                        }
                    }
                }
            }
        }
    }
}

pub async fn fragment_bot_list(State(state): State<Arc<AppState>>) -> Markup {
    bot_list_inner(&state)
}
