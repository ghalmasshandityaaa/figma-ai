use serde::{Deserialize, Serialize};

pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_PAGE_SIZE: u32 = 100;

/// 1-indexed page request, as received from client query params.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PageRequest {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
}

impl PageRequest {
    pub fn page(&self) -> u32 {
        self.page.max(1)
    }

    pub fn page_size(&self) -> u32 {
        if self.page_size == 0 {
            DEFAULT_PAGE_SIZE
        } else {
            self.page_size.clamp(1, MAX_PAGE_SIZE)
        }
    }

    pub fn limit(&self) -> i64 {
        self.page_size() as i64
    }

    pub fn offset(&self) -> i64 {
        (self.page() as i64 - 1) * self.limit()
    }
}

/// Offset-based pagination metadata — snake_case on the wire per API
/// contract. Cursor-based endpoints (none yet) would add `next_cursor`
/// instead of this whole struct.
#[derive(Debug, Serialize)]
pub struct PageMetadata {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub page_count: u32,
    pub page_start: i64,
    pub page_end: i64,
    pub page_current: u32,
    pub page_prev: Option<u32>,
    pub page_next: Option<u32>,
}

impl PageMetadata {
    pub fn new(request: PageRequest, total: i64) -> Self {
        let page = request.page();
        let page_size = request.page_size();
        let page_count = if total == 0 {
            0
        } else {
            (total as f64 / page_size as f64).ceil() as u32
        };
        let page_start = if total == 0 { 0 } else { request.offset() + 1 };
        let page_end = (request.offset() + page_size as i64).min(total).max(page_start);
        let page_prev = (page > 1).then(|| page - 1);
        let page_next = (page < page_count).then_some(page + 1);

        Self {
            total,
            page,
            page_size,
            page_count,
            page_start,
            page_end,
            page_current: page,
            page_prev,
            page_next,
        }
    }
}
