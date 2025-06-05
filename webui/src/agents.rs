use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use serde_json::json;

use std::collections::HashMap;

use crate::WebState;
use crate::data_page::{DataPage, DataPageParams};
use core_logic::datastore::agents::AgentV1;

#[get("/agents?<page>&<range_start>&<filter>&<sort>&<status_filter>")]
pub async fn agents_page(
    page: Option<u32>,
    range_start: Option<u64>,
    filter: Option<String>,
    status_filter: Option<String>,
    sort: Option<String>,
) -> Template {
    Template::render(
        "agents",
        context! {
            sort: sort.unwrap_or_default(),
            range_start: range_start.unwrap_or_default(),
            current_page: page,
            filter: filter.unwrap_or_default(),
            page_name: "Agents",
            status_filter,
        },
    )
}

#[allow(clippy::too_many_arguments)]
#[get("/agents_data?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>&<status_filter>")]
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
