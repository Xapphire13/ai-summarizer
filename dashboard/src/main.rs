use std::sync::{Arc, Mutex};

use maud::{Markup, html};
use rocket::{State, get, launch, post, routes, serde::json::Json};
use serde::Deserialize;

use crate::registry::BotRegistry;

mod registry;

type BotRegistryState = State<Arc<Mutex<BotRegistry>>>;

#[derive(Deserialize)]
struct BotRegistrationRequestData {
    name: String,
}

#[post("/register", format = "json", data = "<data>")]
fn register(data: Json<BotRegistrationRequestData>, bot_registry: &BotRegistryState) -> () {
    bot_registry.lock().unwrap().register(data.0.name);
}

#[get("/")]
fn index(bot_registry: &BotRegistryState) -> Markup {
    let bot_registry = bot_registry.lock().unwrap();
    let bots = bot_registry.bots();

    html! {
        head {
            title { "Dashboard | discord-bots" }
        }

        h1 { "bots/" }

        @if bots.is_empty() {
            p { "No bots registered!" }
        } @else {
            @for bot in bots {
                p { (bot) }
            }
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Arc::new(Mutex::new(BotRegistry::new())))
        .mount("/", routes![index, register])
}
