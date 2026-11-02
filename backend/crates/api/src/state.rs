use std::collections::HashSet;
use std::sync::Arc;

use db::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// E-mails (minúsculos) com papel de superadmin da plataforma.
    pub superadmins: Arc<HashSet<String>>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            superadmins: Arc::default(),
        }
    }

    pub fn with_superadmins<I, S>(mut self, emails: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.superadmins = Arc::new(
            emails
                .into_iter()
                .map(|e| e.as_ref().trim().to_lowercase())
                .filter(|e| !e.is_empty())
                .collect(),
        );
        self
    }

    pub fn is_superadmin(&self, email: &str) -> bool {
        self.superadmins.contains(&email.trim().to_lowercase())
    }
}
