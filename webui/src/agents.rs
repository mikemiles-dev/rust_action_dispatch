use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use serde_json::json;

use crate::WebState;
use crate::data_page::{DataPage, DataPageParams};
use core_logic::datastore::agents::AgentV1;

#[get("/agents?<page>&<range_start>&<filter>&<sort>&<order>")]
pub async fn agents_page(
    state: &State<WebState>,
    page: Option<u32>,
    range_start: Option<u64>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Template {
    let data_page_params = DataPageParams {
        collection: "agents".to_string(),
        range_start,
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
        range_start,
        range_end,
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
