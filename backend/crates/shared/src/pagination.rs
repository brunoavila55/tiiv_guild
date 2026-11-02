use serde::{Deserialize, Serialize};

pub const DEFAULT_PER_PAGE: u32 = 25;
pub const MAX_PER_PAGE: u32 = 100;

/// Parâmetros de paginação vindos da query string (`?page=1&per_page=25`).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PageParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            per_page: default_per_page(),
        }
    }
}

impl PageParams {
    pub fn limit(&self) -> i64 {
        i64::from(self.per_page.clamp(1, MAX_PER_PAGE))
    }

    pub fn offset(&self) -> i64 {
        i64::from(self.page.max(1) - 1) * self.limit()
    }
}

/// Resposta paginada.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_and_offsets() {
        let p = PageParams {
            page: 3,
            per_page: 500,
        };
        assert_eq!(p.limit(), 100);
        assert_eq!(p.offset(), 200);

        let p = PageParams {
            page: 0,
            per_page: 0,
        };
        assert_eq!(p.limit(), 1);
        assert_eq!(p.offset(), 0);
    }
}
