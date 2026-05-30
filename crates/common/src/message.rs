use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub total_items: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub page_size: u64,
    pub next_page_url: String,
    pub prev_page_url: String,
}

impl Pagination {
    pub fn new(
        total_items: u64,
        page_size: u64,
        current_page: u64,
        _url_format: &'static str,
    ) -> Self {
        let total_pages =
            (total_items / page_size) + (!total_items.is_multiple_of(page_size)) as u64;

        let next_page_url = "".to_string();
        let prev_page_url = "".to_string();

        Pagination {
            total_items,
            total_pages,
            current_page,
            page_size,
            next_page_url,
            prev_page_url,
        }
    }
}
