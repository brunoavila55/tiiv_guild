use std::net::SocketAddr;

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    /// `SUPERADMIN_EMAILS`: e-mails separados por vírgula com acesso ao painel da plataforma.
    pub superadmin_emails: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL não definida")?;
        let bind_addr = std::env::var("API_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse()
            .context("API_BIND_ADDR inválido")?;
        let superadmin_emails = std::env::var("SUPERADMIN_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|e| e.trim().to_owned())
            .filter(|e| !e.is_empty())
            .collect();

        Ok(Self {
            database_url,
            bind_addr,
            superadmin_emails,
        })
    }
}
