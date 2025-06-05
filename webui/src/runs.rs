use core_logic::datastore::runs::RunsV1;
use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use serde_json::json;

use crate::WebState;
use crate::data_page::{DataPage, DataPageParams};

#[get("/runs?<page>&<range_start>&<range_end>&<filter>&<sort>&<order>")]
pub async fn runs_page(
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
        range_start,
        range_end,
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
