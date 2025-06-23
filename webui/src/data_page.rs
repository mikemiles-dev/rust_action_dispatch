use bson::{DateTime, doc};
use chrono::{Duration, Utc};
use futures::StreamExt;
use mongodb::options::FindOptions;
use rocket::State;

use std::collections::HashMap;

use crate::WebState;

#[derive(Default, Debug)]
pub struct DataPageParams {
    pub collection: String,
    pub range_field: Option<String>,
    pub range_start: Option<u64>,
    pub range_end: Option<u64>,
    pub search_fields: Vec<String>,
    pub page: Option<u32>,
    pub filter: Option<String>,
    pub additional_filters: Option<HashMap<String, String>>,
    pub sort: Option<String>,
    pub order: Option<String>,
    // New fields for relative selection
    pub relative_select: Option<String>, // "absolute" or "relative"
    pub relative_value: Option<u64>,
    pub relative_unit: Option<String>, // "seconds", "minutes", "hours", "days", "weeks"
}

pub enum RelativeSelect {
    Absolute,
    Relative,
}

impl From<&str> for RelativeSelect {
    fn from(value: &str) -> Self {
        match value {
            "absolute" => RelativeSelect::Absolute,
            "relative" => RelativeSelect::Relative,
            _ => RelativeSelect::Absolute, // Default to absolute if unknown
        }
    }
}

pub struct DataPage<T> {
    pub items: Vec<T>,
    pub total_pages: u64,
    pub current_page: u32,
}

impl<T: Send + Sync + for<'de> serde::Deserialize<'de>> DataPage<T> {
    pub async fn new(state: &State<WebState>, params: DataPageParams) -> DataPage<T> {
        let collection = state
            .datastore
            .get_collection::<T>(&params.collection)
            .await
            .expect("Failed to get collection");

        let page_size = 20;
        let page = params.page.unwrap_or(1);
        let skip = page.saturating_sub(1).saturating_mul(page_size);

        let find_options = Self::build_find_options(&params);

        let mut filter_doc = Self::build_filter(
            params.filter.unwrap_or_default(),
            params.search_fields,
            params.range_field.clone(),
            params.range_start,
            params.range_end,
            params.relative_select.clone(),
            params.relative_value,
            params.relative_unit.clone(),
        );

        if let Some(additional_filters) = &params.additional_filters {
            for (key, value) in additional_filters {
                let addtional_filter_doc = Self::build_filter(
                    value.clone(),
                    vec![key.clone()],
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                filter_doc = doc! {
                    "$and": [filter_doc, addtional_filter_doc]
                };
            }
        }

        let total_count = collection
            .count_documents(filter_doc.clone())
            .await
            .expect("Failed to count documents");
        let total_pages = total_count.div_ceil(page_size as u64);

        let mut cursor = collection
            .find(filter_doc)
            .with_options(find_options)
            .skip(skip as u64)
            .limit(page_size as i64)
            .await
            .expect("Failed to fetch data");

        let mut items = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => items.push(doc),
                Err(e) => eprintln!("Error reading document: {:?}", e),
            }
        }

        DataPage {
            items,
            total_pages,
            current_page: page,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_filter(
        filter_str: String,
        search_fields: Vec<String>,
        range_field: Option<String>,
        range_start: Option<u64>,
        range_end: Option<u64>,
        relative_select: Option<String>,
        relative_value: Option<u64>,
        relative_unit: Option<String>,
    ) -> bson::Document {
        let mut filter = if !filter_str.trim().is_empty() {
            let regex = doc! { "$regex": &filter_str, "$options": "i" };
            let mut or_conditions: Vec<_> = search_fields
                .iter()
                .map(|field| doc! { field: regex.clone() })
                .collect();

            if let Ok(num_val_i32) = filter_str.parse::<i32>() {
                for field in &search_fields {
                    or_conditions.push(doc! { field: num_val_i32 });
                }
            } else if let Ok(num_val_i64) = filter_str.parse::<i64>() {
                for field in &search_fields {
                    or_conditions.push(doc! { field: num_val_i64 });
                }
            } else if let Ok(num_val_f64) = filter_str.parse::<f64>() {
                for field in &search_fields {
                    or_conditions.push(doc! { field: num_val_f64 });
                }
            }
            doc! { "$or": or_conditions }
        } else {
            doc! {}
        };

        let relative_select: RelativeSelect =
            relative_select.as_deref().unwrap_or("absolute").into();

        println!("RANGE FIELD: {:?}", range_field);

        // Handle additional filters
        if let Some(field) = range_field {
            let mut range_doc = doc! {};
            if matches!(relative_select, RelativeSelect::Relative) {
                if let (Some(value), Some(unit)) = (relative_value, relative_unit) {
                    let duration = match unit.as_str() {
                        "seconds" => Duration::seconds(value as i64),
                        "minutes" => Duration::minutes(value as i64),
                        "hours" => Duration::hours(value as i64),
                        "days" => Duration::days(value as i64),
                        "weeks" => Duration::weeks(value as i64),
                        _ => Duration::seconds(0),
                    };
                    let now = Utc::now();
                    let start = now - duration;
                    range_doc.insert("$gte", DateTime::from_millis(start.timestamp_millis()));
                    range_doc.insert("$lte", DateTime::from_millis(now.timestamp_millis()));
                }
            } else {
                if let Some(range_start) = range_start {
                    range_doc.insert("$gte", DateTime::from_millis(range_start as i64));
                }
                if let Some(range_end) = range_end {
                    range_doc.insert("$lte", DateTime::from_millis(range_end as i64));
                }
            }
            println!("BBB range_doc: {:?}", range_doc);
            if !range_doc.is_empty() {
                filter.insert(field, range_doc);
            }
        }

        filter
    }

    fn build_find_options(params: &DataPageParams) -> FindOptions {
        let mut options = FindOptions::default();
        if let Some(ref sort_field) = params.sort {
            let sort_order = match params.order.as_deref() {
                Some("desc") => -1,
                _ => 1,
            };
            options.sort = Some(doc! { sort_field: sort_order });
        }
        options
    }
}
