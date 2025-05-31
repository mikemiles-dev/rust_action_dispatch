use bson::DateTime;
use futures::StreamExt;
use mongodb::options::FindOptions;
use rocket::State;

use crate::WebState;

pub struct DataPageParams {
    pub collection: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
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
            collection,
            range_start,
            range_end,
            search_fields: sort_fields,
            page,
            filter,
            sort,
            order,
        } = data_page_params;

        let runs_future = { state.datastore.get_collection::<T>(&collection) };
        let collection = runs_future.await.expect("Failed to get runs collection");
        let mut bson_filter = bson::doc! {};

        let page_size = 20;
        let page = page.unwrap_or(1);
        let skip = page.saturating_sub(1).saturating_mul(page_size);

        // Apply sorting if provided
        let mut find_options = FindOptions::default();
        if let Some(sort_field) = sort {
            // Determine sort order: 1 for ascending, -1 for descending
            // If a filter is provided, build a $or query to search all string fields
            bson_filter = if let Some(ref filter) = filter {
                // List the fields you want to search
                let regex = bson::doc! { "$regex": filter, "$options": "i" };
                let or_conditions: Vec<_> = sort_fields
                    .iter()
                    .map(|field| bson::doc! { field: regex.clone() })
                    .collect();
                bson::doc! { "$or": or_conditions }
            } else {
                bson::doc! {}
            };

            let sort_order = match order.as_deref() {
                Some("desc") => -1,
                _ => 1,
            };
            find_options.sort = Some(bson::doc! { sort_field: sort_order });
        }

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
            .expect("Failed to find runs");
        let mut runs = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => runs.push(T::from(doc)),
                Err(e) => eprintln!("Error reading run: {:?}", e),
            }
        }

        DataPage {
            items: runs,
            total_pages,
            current_page: page,
        }
    }
}
