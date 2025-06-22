mod agents;
mod data_page;
mod runs;

use rocket::fs::NamedFile;
use rocket::fs::{FileServer, relative};
use rocket::get;
use rocket::http::Status;
use rocket::response::{Responder, status::Custom};
use rocket::routes;
use rocket::{Catcher, Request, catcher};
use rocket_dyn_templates::{Template, context, minijinja::Environment};

use std::env;
use std::path::{Path, PathBuf};

use agents::{add_agent, agents_data, agents_page, delete_agent, edit_agent, post_agents};
use core_logic::datastore::Datastore;
use runs::{runs_data, runs_output, runs_page};

use crate::agents::delete_agents_bulk;

pub struct WebState {
    datastore: Datastore,
}

#[get("/")]
pub fn index() -> Template {
    Template::render(
        "index",
        context! {
            title: "Dashboard",
        },
    )
}

#[rocket::get("/static/<path..>")]
pub async fn static_files(path: PathBuf) -> Option<NamedFile> {
    let path = Path::new(relative!("static")).join(path);
    NamedFile::open(path).await.ok()
}

fn not_found_handler<'r>(_: Status, req: &'r Request) -> catcher::BoxFuture<'r> {
    let responder = Custom(Status::NotFound, format!("Couldn't find: {}", req.uri()));
    Box::pin(async move { responder.respond_to(req) })
}

pub fn customize(_env: &mut Environment) {}

#[rocket::launch]
async fn rocket() -> _ {
    let not_found_catcher = Catcher::new(404, not_found_handler);

    let web_state = WebState {
        datastore: Datastore::try_new()
            .await
            .expect("Failed to initialize datastore"),
    };
    // Read port from environment variable or default to 8000
    let port: u16 = env::var("WEBUI_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000);

    let figment = rocket::Config::figment().merge(("port", port));

    rocket::build()
        .configure(rocket::Config::from(figment))
        .manage(web_state)
        .mount(
            "/",
            routes![
                index,
                runs_page,
                runs_output,
                agents_page,
                edit_agent,
                runs_data,
                agents_data,
                post_agents,
                add_agent,
                delete_agent,
                delete_agents_bulk,
            ],
        )
        .mount("/", rocket::routes![static_files])
        .mount(
            "/",
            FileServer::new(relative!("static"), rocket::fs::Options::default()),
        )
        .register("/", vec![not_found_catcher])
        .attach(Template::custom(|engines| {
            customize(&mut engines.minijinja);
        }))
}
