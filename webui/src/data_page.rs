use bson::{DateTime, doc};
use futures::StreamExt;
use mongodb::options::FindOptions;
use rocket::State;

use crate::WebState;

pub struct DataPageParams {
    pub collection: String,
    pub range_start: Option<u64>,
    pub range_end: Option<u64>,
    pub search_fields: Vec<String>,
    pub page: Option<u32>,
    pub filter: Option<String>,
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
    pub async fn new(state: &State<WebState>, data_page_params: DataPageParams) -> DataPage<T> {
        let DataPageParams {
            collection,    // Collection name
            range_start,   // Optional start of the date range
            range_end,     // Optional end of the date range
            search_fields, // Fields to search in the filter
            filter,        // Optional filter string
            page,          // Page number for pagination
            sort,          // Optional field to sort by
            order,         // Optional order for sorting (asc/desc)
        } = data_page_params;

        let filter = match filter {
            Some(filter) if !filter.trim().is_empty() => Some(filter),
            _ => None,
        };

        let store_future = { state.datastore.get_collection::<T>(&collection) };
        let collection = store_future.await.expect("Failed to get runs collection");

        let page_size = 20;
        let page = page.unwrap_or(1);
        let skip = page.saturating_sub(1).saturating_mul(page_size);

        // Apply sorting if provided
        let mut find_options = FindOptions::default();
        // Determine sort order: 1 for ascending, -1 for descending
        // If a filter is provided, build a $or query to search all string fields
        let mut bson_filter = if let Some(ref filter) = filter {
            // List the fields you want to search
            let regex = doc! { "$regex": filter, "$options": "i" };
            let mut or_conditions: Vec<_> = search_fields
                .iter()
                .map(|field| doc! { field: regex.clone() })
                .collect();

            // 2. Attempt to parse the filter as a number (i32 or i64 for common cases)
            if let Ok(num_val_i32) = filter.parse::<i32>() {
                for field in search_fields.iter() {
                    // Add a condition to match the numeric value
                    or_conditions.push(doc! { field: num_val_i32 });
                }
            } else if let Ok(num_val_i64) = filter.parse::<i64>() {
                for field in search_fields.iter() {
                    // Add a condition to match the numeric value
                    or_conditions.push(doc! { field: num_val_i64 });
                }
            }
            // You could also consider f64 for floating-point numbers if applicable
            else if let Ok(num_val_f64) = filter.parse::<f64>() {
                for field in search_fields.iter() {
                    or_conditions.push(doc! { field: num_val_f64 });
                }
            }

            doc! { "$or": or_conditions }
        } else {
            doc! {}
        };

        if let Some(sort_field) = sort {
            let sort_order = match order.as_deref() {
                Some("desc") => -1,
                _ => 1,
            };
            find_options.sort = Some(doc! { sort_field: sort_order });
        }

        if let Some(range_start) = &range_start {
            bson_filter.insert(
                "started_at",
                doc! { "$gte": DateTime::from_millis(*range_start as i64) },
            );
        }
        if let Some(range_end) = &range_end {
            bson_filter.insert(
                "completed_at",
                doc! { "$lte": DateTime::from_millis(*range_end as i64) },
            );
        }

        println!("Using filter: {:?}", bson_filter);

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
            .expect("Failed to data");
        let mut items = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => items.push(T::from(doc)),
                Err(e) => eprintln!("Error reading run: {:?}", e),
            }
        }

        DataPage {
            items,
            total_pages,
            current_page: page,
        }
    }
}
