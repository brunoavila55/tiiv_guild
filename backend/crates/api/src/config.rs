use anyhow::Context;

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
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let base_domain = var("BASE_DOMAIN")
            .context("defina BASE_DOMAIN (ex.: tiiv.com.br)")?
            .to_ascii_lowercase();
        Ok(Self {
            database_url: var("DATABASE_URL").context("defina DATABASE_URL")?,
            bind_addr: var("API_BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            cookie_domain: Some(var("COOKIE_DOMAIN").unwrap_or_else(|| base_domain.clone())),
            base_domain,
            cookie_secure: var("COOKIE_SECURE").is_none_or(|v| v != "false"),
            superadmin_emails: parse_emails(var("SUPERADMIN_EMAILS").as_deref().unwrap_or("")),
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
