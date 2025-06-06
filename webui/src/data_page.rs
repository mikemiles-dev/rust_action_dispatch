use bson::{DateTime, doc};
use futures::StreamExt;
use mongodb::options::FindOptions;
use rocket::State;

use std::collections::HashMap;

use crate::WebState;

#[derive(Default)]
pub struct DataPageParams {
    pub collection: String,
    pub range_start_key: Option<String>, // for future use
    pub range_end_key: Option<String>,   // for future use
    pub range_start: Option<u64>,
    pub range_end: Option<u64>,
    pub search_fields: Vec<String>,
    pub page: Option<u32>,
    pub filter: Option<String>,
    pub additional_filters: Option<HashMap<String, String>>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

pub struct DataPage<T> {
    pub items: Vec<T>,
    pub total_pages: u64,
    pub current_page: u32,
}

impl<T: Send + Sync + for<'de> serde::Deserialize<'de>> DataPage<T> {
    /// Fetch a paginated list of items from a MongoDB collection
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
            params.range_start_key.clone(),
            params.range_end_key.clone(),
            params.range_start,
            params.range_end,
        );

        if let Some(additional_filters) = &params.additional_filters {
            for (key, value) in additional_filters {
                let addtional_filter_doc =
                    Self::build_filter(value.clone(), vec![key.clone()], None, None, None, None);
                filter_doc.extend(addtional_filter_doc);
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

    fn build_filter(
        filter_str: String,
        search_fields: Vec<String>,
        range_start_key: Option<String>,
        range_end_key: Option<String>,
        range_start: Option<u64>,
        range_end: Option<u64>,
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

        if let Some(range_start) = range_start {
            filter.insert(
                range_start_key.unwrap_or("started_at".to_string()),
                doc! { "$gte": DateTime::from_millis(range_start as i64) },
            );
        }
        if let Some(range_end) = range_end {
            filter.insert(
                range_end_key.unwrap_or("completed_at".to_string()),
                doc! { "$lte": DateTime::from_millis(range_end as i64) },
            );
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
