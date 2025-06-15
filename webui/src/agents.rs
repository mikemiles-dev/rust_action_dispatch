use mongodb::bson::{doc, oid::ObjectId};
use rocket::State;
use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use rocket::{get, post};
use rocket_dyn_templates::{Template, context};
use serde_json::json;

use std::collections::HashMap;

use crate::WebState;
use crate::data_page::{DataPage, DataPageParams};
use core_logic::datastore::agents::AgentV1;

#[derive(FromForm, Debug)]
pub struct AgentForm {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub port: u16,
}

#[post("/agents", data = "<form>")]
pub async fn post_agents(
    state: &State<WebState>,
    form: Form<AgentForm>,
) -> Result<String, (rocket::http::Status, String)> {
    println!("Received form: {:?}", form);
    let agent_collection = state
        .datastore
        .get_collection::<AgentV1>("agents")
        .await
        .map_err(|e| {
            (
                rocket::http::Status::InternalServerError,
                format!("Error accessing agents collection: {}", e),
            )
        })?;

    if form.id.is_empty() {
        let new_agent = AgentV1 {
            name: form.name.clone(),
            hostname: form.hostname.clone(),
            port: form.port,
            ..Default::default()
        };
        agent_collection.insert_one(new_agent).await.map_err(|e| {
            (
                rocket::http::Status::InternalServerError,
                format!("Error inserting agent: {}", e),
            )
        })?;
    } else {
        let object_id = ObjectId::parse_str(&form.id).map_err(|_| {
            (
                rocket::http::Status::BadRequest,
                "Invalid agent ID format".to_string(),
            )
        })?;
        let agent = agent_collection
            .find_one(doc! { "_id": object_id })
            .await
            .map_err(|e| {
                (
                    rocket::http::Status::InternalServerError,
                    format!("Error fetching agent: {}", e),
                )
            })?;
        agent.ok_or((
            rocket::http::Status::NotFound,
            "Agent not found".to_string(),
        ))?;
        let update_doc = doc! {
            "$set": {
                "name": &form.name,
                "hostname": &form.hostname,
                "port": form.port as i32,
            }
        };
        agent_collection
            .update_one(doc! { "_id": &object_id }, update_doc)
            .await
            .map_err(|e| {
                (
                    rocket::http::Status::InternalServerError,
                    format!("Error updating agent: {}", e),
                )
            })?;
    };

    Ok("Success".to_string())
}

#[get("/agents?<page>&<range_start>&<range_end>&<filter>&<sort>&<status_filter>")]
pub async fn agents_page(
    page: Option<u32>,
    range_start: Option<u64>,
    range_end: Option<u64>, // range_end is not used in agents_page, but required for data_page
    filter: Option<String>,
    status_filter: Option<String>,
    sort: Option<String>,
) -> Template {
    Template::render(
        "agents",
        context! {
            sort: sort.unwrap_or_default(),
            range_start: range_start.unwrap_or_default(),
            range_end: range_end.unwrap_or_default(),
            current_page: page,
            filter: filter.unwrap_or_default(),
            page_name: "Agents",
            status_filter,
        },
    )
}

#[allow(clippy::too_many_arguments)]
#[get("/agents/data?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>&<status_filter>")]
pub async fn agents_data(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    status_filter: Option<String>,
) -> Json<serde_json::Value> {
    let data_page_params = DataPageParams {
        collection: "agents".to_string(),
        range_end_key: Some("last_ping".to_string()), // for sorting by last_ping
        range_start_key: Some("last_ping".to_string()), // for sorting by last_ping
        range_start,
        range_end,
        search_fields: vec![
            "name".to_string(),
            "hostname".to_string(),
            "last_ping".to_string(),
            "status".to_string(),
            "port".to_string(),
        ],
        additional_filters: if status_filter.is_some() {
            let mut filters = HashMap::new();
            filters.insert("status".to_string(), status_filter.unwrap());
            Some(filters)
        } else {
            None
        },
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

#[get("/agents/edit?<id>")]
pub async fn edit_agent(state: &State<WebState>, id: &str) -> Template {
    let render = |error: &str, agent: Option<AgentV1>| {
        Template::render(
            "edit_agent",
            context! {
                page_name: "Edit Agent",
                agent_id: id.to_string(),
                agent,
                error: error.to_string(),
            },
        )
    };

    let agent_collection = match state.datastore.get_collection::<AgentV1>("agents").await {
        Ok(coll) => coll,
        Err(_) => return render("Failed to access agents collection", None),
    };

    let object_id = match ObjectId::parse_str(id) {
        Ok(oid) => oid,
        Err(_) => return render("Invalid agent ID format", None),
    };

    match agent_collection.find_one(doc! { "_id": object_id }).await {
        Ok(Some(agent)) => render("", Some(agent)),
        Ok(None) => render("Agent not found", None),
        Err(e) => render(&format!("Error fetching agent: {}", e), None),
    }
}

#[get("/agents/add")]
pub async fn add_agent(_state: &State<WebState>) -> Template {
    Template::render(
        "edit_agent",
        context! {
            page_name: "Add Agent",
        },
    )
}

#[get("/agents/delete?<id>")]
pub async fn delete_agent(
    state: &State<WebState>,
    id: &str,
) -> Result<Template, (rocket::http::Status, String)> {
    let agent_collection = state
        .datastore
        .get_collection::<AgentV1>("agents")
        .await
        .map_err(|e| {
            (
                rocket::http::Status::InternalServerError,
                format!("Error accessing agents collection: {}", e),
            )
        })?;

    let object_id = ObjectId::parse_str(id).map_err(|_| {
        (
            rocket::http::Status::BadRequest,
            "Invalid agent ID format".to_string(),
        )
    })?;

    agent_collection
        .delete_one(doc! { "_id": object_id })
        .await
        .map_err(|e| {
            (
                rocket::http::Status::InternalServerError,
                format!("Error deleting agent: {}", e),
            )
        })?;

    Ok(Template::render(
        "delete_success",
        context! {
            page_name: "Delete Agent",
            message: "Agent deleted successfully",
        },
    ))
}
