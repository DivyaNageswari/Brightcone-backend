use ts_rs::TS;

/// A page of results with pagination metadata.
#[derive(serde::Serialize, serde::Deserialize, TS)]
pub struct Page<T: TS> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub page: u64,
    #[ts(type = "number")]
    pub page_size: u64,
    #[ts(type = "number")]
    pub total_pages: u64,
    #[ts(type = "number")]
    pub total_items: u64,
}

impl<T: TS> Page<T> {
    pub fn new(
        items: Vec<T>,
        page: u64,
        page_size: u64,
        total_pages: u64,
        total_items: u64,
    ) -> Self {
        Self {
            items,
            page,
            page_size,
            total_pages,
            total_items,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[ts(type = "unknown")]
    pub details: Option<serde_json::Value>,
}
