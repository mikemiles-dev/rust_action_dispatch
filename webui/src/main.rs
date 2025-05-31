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

use rocket_dyn_templates::{
    Template, context,
    minijinja::{self, Environment},
};

use core_logic::datastore::Datastore;
use mongodb::options::FindOptions;

use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

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
            title: "Dashboard",
        },
    )
}

#[get("/runs?<page>&<filter>&<sort>&<order>")]
pub async fn runs(
    state: &State<WebState>,
    page: Option<u32>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Template {
    let runs_future = { state.datastore.get_collection::<RunsV1>("runs") };
    let collection = runs_future.await.expect("Failed to get runs collection");
    let mut bson_filter = bson::doc! {};

    let page_size = 20;
    let page = page.unwrap_or(1);
    let skip = page.saturating_sub(1).saturating_mul(page_size);

    // Apply sorting if provided
    let mut find_options = FindOptions::default();
    if let Some(sort_field) = sort {
        // Determine sort order: 1 for ascending, -1 for descending
        // If a filter is provided, build a $or query to search all string fields
        bson_filter = if let Some(ref filter) = filter {
            // List the fields you want to search
            let search_fields = ["job_name", "agent_name"];
            let regex = bson::doc! { "$regex": filter, "$options": "i" };
            let or_conditions: Vec<_> = search_fields
                .iter()
                .map(|field| bson::doc! { *field: regex.clone() })
                .collect();
            bson::doc! { "$or": or_conditions }
        } else {
            bson::doc! {}
        };

        let sort_order = match order.as_deref() {
            Some("desc") => -1,
            _ => 1,
        };
        find_options.sort = Some(bson::doc! { sort_field: sort_order });
    }

    // Count total documents for pagination
    let total_count = collection
        .count_documents(bson_filter.clone())
        .await
        .expect("Failed to count documents");

    let total_pages = total_count.div_ceil(page_size as u64);

    let mut cursor = collection
        .find(bson_filter.clone())
        .with_options(find_options)
        .skip(skip as u64)
        .limit(page_size as i64)
        .await
        .expect("Failed to find runs");
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
            total_pages,
            current_page: page,
            page_name: "Runs",
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
