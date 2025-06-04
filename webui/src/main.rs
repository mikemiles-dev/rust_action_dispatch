mod data_page;

use core_logic::datastore::agents::AgentV1;
use core_logic::datastore::runs::RunsV1;
use rocket::State;
use rocket::fs::NamedFile;
use rocket::fs::{FileServer, relative};
use rocket::get;
use rocket::http::Status;
use rocket::response::{Responder, status::Custom};
use rocket::routes;
use rocket::serde::json::Json;
use rocket::{Catcher, Request, catcher};
use rocket_dyn_templates::{Template, context, minijinja::Environment};
use serde_json::json;

use std::env;
use std::path::{Path, PathBuf};

use core_logic::datastore::Datastore;
use data_page::{DataPage, DataPageParams};

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

#[get("/runs?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>")]
pub async fn runs(
    range_start: Option<u64>,
    range_end: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<u32>,
) -> Template {
    Template::render(
        "runs",
        context! {
            sort: sort.unwrap_or_default(),
            page: page.unwrap_or(1),
            range_start: range_start.unwrap_or_default(),
            range_end: range_end.unwrap_or_default(),
            filter: filter.unwrap_or_default(),
            order: order.unwrap_or_default(),
            page_name: "Runs",
        },
    )
}

#[get("/agents_data?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>")]
pub async fn agents_data(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Json<serde_json::Value> {
    let data_page_params = DataPageParams {
        collection: "agents".to_string(),
        range_start: range_start.clone(),
        range_end: range_end.clone(),
        search_fields: vec![
            "name".to_string(),
            "hostname".to_string(),
            "last_ping".to_string(),
            "status".to_string(),
            "port".to_string(),
        ],
        page,
        filter: filter.clone(),
        sort: sort.clone(),
        order,
    };

    let runs_page: DataPage<AgentV1> = DataPage::new(state, data_page_params).await;

    let DataPage {
        items: runs,
        total_pages,
        current_page: page,
    } = runs_page;

    Json(json!({
        "items": runs,
        "total_pages": total_pages,
        "current_page": page,
    }))
}

#[get("/runs_data?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>")]
pub async fn runs_data(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Json<serde_json::Value> {
    let data_page_params = DataPageParams {
        collection: "runs".to_string(),
        range_start: range_start.clone(),
        range_end: range_end.clone(),
        search_fields: vec![
            "job_name".to_string(),
            "agent_name".to_string(),
            "return_code".to_string(),
        ],
        page,
        filter: filter.clone(),
        sort: sort.clone(),
        order,
    };

    let runs_page: DataPage<RunsV1> = DataPage::new(state, data_page_params).await;

    let DataPage {
        items: runs,
        total_pages,
        current_page: page,
    } = runs_page;

    Json(json!({
        "items": runs,
        "total_pages": total_pages,
        "current_page": page,
    }))
}

#[get("/agents?<page>&<range_start>&<filter>&<sort>&<order>")]
pub async fn agents(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Template {
    let data_page_params = DataPageParams {
        collection: "agents".to_string(),
        range_start: range_start.clone(),
        range_end: None,
        search_fields: vec![
            "name".to_string(),
            "hostname".to_string(),
            "port".to_string(),
        ],
        page,
        filter: filter.clone(),
        sort: sort.clone(),
        order,
    };

    let runs_page: DataPage<AgentV1> = DataPage::new(state, data_page_params).await;

    let DataPage {
        items: runs,
        total_pages,
        current_page: page,
    } = runs_page;

    Template::render(
        "agents",
        context! {
            items: runs,
            sort: sort.unwrap_or_default(),
            range_start: range_start.unwrap_or_default(),
            total_pages,
            current_page: page,
            filter: filter.unwrap_or_default(),
            page_name: "Agents",
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
        .mount("/", routes![index, runs, agents, runs_data, agents_data])
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
