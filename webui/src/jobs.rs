use core_logic::datastore::jobs::JobV1;
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
    "/jobs?<page>&<range_select>&<relative_select>&<relative_select_value>&<relative_select_unit>&<range_start>&<range_end>&<filter>&<outcome_filter>&<sort>&<order>"
)]
pub async fn jobs_page(
    range_start: Option<u64>,
    range_end: Option<u64>,
    range_select: Option<String>,
    relative_select: Option<String>,
    relative_select_value: Option<u8>,
    relative_select_unit: Option<String>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    outcome_filter: Option<String>,
    page: Option<u32>,
) -> Template {
    Template::render(
        "jobs",
        context! {
            sort: sort.unwrap_or_default(),
            page: page.unwrap_or(1),
            range_start: range_start.unwrap_or_default(),
            range_end: range_end.unwrap_or_default(),
            range_select: range_select.unwrap_or_default(),
            range_fields: vec!["started_at".to_string(), "completed_at".to_string()], // Assuming these are the fields for range filtering
            filter: filter.unwrap_or_default(),
            order: order.unwrap_or_default(),
            outcome_filter: outcome_filter.unwrap_or_default(),
            page_name: "Jobs",
            relative_select: relative_select.unwrap_or_default(),
            relative_select_value: relative_select_value.unwrap_or(30),
            relative_select_unit: relative_select_unit.unwrap_or_default(),
        },
    )
}

#[allow(clippy::too_many_arguments)]
#[get(
    "/jobs_data?<page>&<range_select>&<relative_select>&<relative_select_value>&<relative_select_unit>&<range_start>&<range_end>&<filter>&<sort>&<outcome_filter>&<order>"
)]
pub async fn jobs_data(
    state: &State<WebState>,
    page: Option<u32>,
    range_select: Option<String>,
    range_end: Option<u64>,
    range_start: Option<u64>,
    relative_select: Option<String>,
    relative_select_value: Option<u8>,
    relative_select_unit: Option<String>,
    filter: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    outcome_filter: Option<String>,
) -> Json<serde_json::Value> {
    let range_select = range_select
        .clone()
        .unwrap_or_else(|| "started_at".to_string());
    let data_page_params = DataPageParams {
        collection: "jobs".to_string(),
        range_start,
        range_end,
        range_field: Some(range_select),
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
        relative_select,
        relative_value: relative_select_value.map(|v| v as u64),
        relative_unit: relative_select_unit,
    };

    let jobs_page: DataPage<JobV1> = DataPage::new(state, data_page_params).await;

    let DataPage {
        items: jobs,
        total_pages,
        current_page: page,
    } = jobs_page;

    Json(json!({
        "items": jobs,
        "total_pages": total_pages,
        "current_page": page,
    }))
}
