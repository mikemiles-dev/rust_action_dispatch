use core_logic::datastore::runs::RunsV1;
use rocket::State;
use rocket::data::{Data, ToByteUnit};
use rocket::fs::NamedFile;
use rocket::fs::{FileServer, relative};
use rocket::http::{
    Method::{Get, Post},
    Status,
};
use rocket::outcome::{IntoOutcome, try_outcome};
use rocket::response::Redirect;
use rocket::response::{Responder, status::Custom};
use rocket::routes;
use rocket::tokio::fs::File;
use rocket::{Catcher, Request, Route, catcher, route};
use rocket::{get, uri};

use futures::StreamExt;
use mongodb::{Client, Database};

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use rocket_dyn_templates::{
    Template, context,
    minijinja::{self, Environment},
};

use core_logic::datastore::Datastore;

struct WebState {
    datastore: Datastore,
}

// fn forward<'r>(_req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
//     Box::pin(async move { route::Outcome::forward(data, Status::NotFound) })
// }

// fn hi<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
//     route::Outcome::from(req, "Hello!").pin()
// }

#[get("/")]
pub fn index() -> Template {
    Template::render(
        "index",
        context! {
            title: "Hello",
            name: Some("blah"),
            items: vec!["One", "Two", "Three"],
        },
    )
}

#[get("/runs")]
pub async fn runs(state: &State<WebState>) -> Template {
    let runs_future = { state.datastore.get_collection::<RunsV1>("runs") };
    let collection = runs_future.await.expect("Failed to get runs collection");
    let filter = bson::doc! {};

    let mut cursor = collection.find(filter).await.expect("Failed to find runs");

    let mut runs = Vec::new();
    while let Some(result) = cursor.next().await {
        match result {
            Ok(doc) => runs.push(RunsV1::from(doc)),
            Err(e) => eprintln!("Error reading run: {:?}", e),
        }
    }
    Template::render(
        "runs",
        context! {
            items: runs,
            page_name: "Hello",
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

pub fn customize(env: &mut Environment) {
    env.add_template(
        "minijinja/about.html",
        r#"
        {% extends "minijinja/layout" %}

        {% block page %}
            <section id="about">
                <h1>About - Here's another page!</h1>
            </section>
        {% endblock %}
    "#,
    )
    .expect("valid Jinja2 template");
}

#[rocket::launch]
async fn rocket() -> _ {
    let not_found_catcher = Catcher::new(404, not_found_handler);

    // let echo = Route::new(Get, "/echo/<str>", echo_url);
    // let name = Route::new(Get, "/<name>", name);
    // let post_upload = Route::new(Post, "/", upload);
    // let get_upload = Route::new(Get, "/", get_upload);

    let web_state = WebState {
        datastore: Datastore::try_new()
            .await
            .expect("Failed to initialize datastore"),
    };

    rocket::build()
        .manage(web_state)
        .mount("/", routes![index, runs])
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
