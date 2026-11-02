use anyhow::Context;

use crate::media::StorageConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    /// Domínio raiz dos tenants: `{slug}.BASE_DOMAIN`.
    pub base_domain: String,
    /// `Domain` do cookie de sessão; por padrão o `base_domain`, para valer em todo subdomínio.
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
    /// E-mails (minúsculos) com acesso ao painel da plataforma.
    pub superadmin_emails: Vec<String>,
    pub storage: StorageConfig,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let required = |k: &str| var(k).with_context(|| format!("defina {k}"));
        let base_domain = var("BASE_DOMAIN")
            .context("defina BASE_DOMAIN (ex.: tiiv.com.br)")?
            .to_ascii_lowercase();
        let storage = match var("STORAGE").as_deref().unwrap_or("local") {
            "local" => StorageConfig::Local {
                dir: var("STORAGE_DIR")
                    .unwrap_or_else(|| "data/media".into())
                    .into(),
            },
            "s3" => StorageConfig::S3 {
                bucket: required("S3_BUCKET")?,
                endpoint: required("S3_ENDPOINT")?,
                region: var("S3_REGION").unwrap_or_else(|| "us-east-1".into()),
                access_key_id: required("S3_ACCESS_KEY_ID")?,
                secret_access_key: required("S3_SECRET_ACCESS_KEY")?,
                public_url: required("S3_PUBLIC_URL")?,
            },
            other => anyhow::bail!("STORAGE inválido: {other} (use local ou s3)"),
        };
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: var("API_BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            cookie_domain: Some(var("COOKIE_DOMAIN").unwrap_or_else(|| base_domain.clone())),
            base_domain,
            cookie_secure: var("COOKIE_SECURE").is_none_or(|v| v != "false"),
            superadmin_emails: parse_emails(var("SUPERADMIN_EMAILS").as_deref().unwrap_or("")),
            storage,
        })
    }

    pub fn is_superadmin(&self, email: &str) -> bool {
        self.superadmin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(email))
    }
}

pub fn parse_emails(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|e| e.trim().to_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}
