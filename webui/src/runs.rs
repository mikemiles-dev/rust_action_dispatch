use core_logic::datastore::runs::RunsV1;
use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use serde_json::json;

use std::collections::HashMap;

use crate::WebState;
use crate::data_page::{DataPage, DataPageParams};

#[allow(clippy::too_many_arguments)]
#[get(
    "/runs?<page>&<range_select>&<range_start>&<range_end>&<filter>&<outcome_filter>&<sort>&<order>"
)]
pub async fn runs_page(
    range_start: Option<u64>,
    range_end: Option<u64>,
    range_select: Option<String>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    outcome_filter: Option<String>,
    page: Option<u32>,
) -> Template {
    Template::render(
        "runs",
        context! {
            sort: sort.unwrap_or_default(),
            page: page.unwrap_or(1),
            range_start: range_start.unwrap_or_default(),
            range_end: range_end.unwrap_or_default(),
            range_select: range_select.unwrap_or_default(),
            range_fields: vec!["start_time".to_string(), "end_time".to_string()], // Assuming these are the fields for range filtering
            filter: filter.unwrap_or_default(),
            order: order.unwrap_or_default(),
            outcome_filter: outcome_filter.unwrap_or_default(),
            page_name: "Runs",
        },
    )
}

#[get("/runs_output?<id>")]
pub async fn runs_output(state: &State<WebState>, id: Option<String>) -> String {
    let collection = match state.datastore.get_collection::<RunsV1>("runs").await {
        Ok(coll) => coll,
        Err(_) => {
            return "Error retrieving runs collection".to_string();
        }
    };
    let object_id = match mongodb::bson::oid::ObjectId::parse_str(id.unwrap_or_default()) {
        Ok(oid) => oid,
        Err(e) => {
            println!("Error parsing ObjectId: {}", e);
            return "Invalid ObjectId format".to_string();
        }
    };
    let run_entry = match collection
        .find_one(mongodb::bson::doc! { "_id": object_id })
        .await
    {
        Ok(Some(entry)) => entry,
        _ => {
            return "Run entry not found".to_string();
        }
    };
    run_entry.output
}

#[allow(clippy::too_many_arguments)]
#[get("/runs_data?<page>&<range_start>&<range_end>&<filter>&<sort>&<outcome_filter>&<order>")]
pub async fn runs_data(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    outcome_filter: Option<String>,
) -> Json<serde_json::Value> {
    let data_page_params = DataPageParams {
        collection: "runs".to_string(),
        range_start,
        range_end,
        search_fields: vec![
            "job_name".to_string(),
            "agent_name".to_string(),
            "return_code".to_string(),
            "command".to_string(),
            "output".to_string(),
        ],
        page,
        filter: filter.clone(),
        additional_filters: if outcome_filter.is_some() {
            let mut filters = HashMap::new();
            filters.insert("outcome".to_string(), outcome_filter.unwrap());
            Some(filters)
        } else {
            None
        },
        sort: sort.clone(),
        order,
        ..Default::default()
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
