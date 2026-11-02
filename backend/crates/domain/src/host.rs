//! Host da requisição → contexto. Espelhado em `frontend/src/app/host.ts`.

use crate::Error;

/// Subdomínios que nunca podem virar slug de guilda.
pub const RESERVED_SUBDOMAINS: &[&str] = &[
    "admin", "api", "app", "assets", "auth", "blog", "cdn", "dev", "docs", "help", "id", "login",
    "mail", "media", "staging", "static", "status", "suporte", "support", "test", "www",
];

pub const SLUG_MIN: usize = 3;
pub const SLUG_MAX: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKind {
    /// `BASE_DOMAIN` ou `www.`: landing.
    Root,
    /// `app.`: conta, login, criar guilda.
    App,
    /// `admin.`: painel da plataforma.
    Admin,
    /// `{slug}.`: uma guilda (que pode não existir).
    Tenant(String),
    /// Fora de `BASE_DOMAIN` ou subdomínio inválido.
    Unknown,
}

/// Classifica um header `Host` (com ou sem porta) em relação a `base_domain`.
pub fn classify(host: &str, base_domain: &str) -> HostKind {
    let host = strip_port(host).trim_end_matches('.').to_ascii_lowercase();
    let base = base_domain.trim_end_matches('.').to_ascii_lowercase();
    if host == base {
        return HostKind::Root;
    }
    let Some(sub) = host
        .strip_suffix(&base)
        .and_then(|rest| rest.strip_suffix('.'))
    else {
        return HostKind::Unknown;
    };
    match sub {
        "www" => HostKind::Root,
        "app" => HostKind::App,
        "admin" => HostKind::Admin,
        s if validate_slug(s).is_ok() => HostKind::Tenant(s.to_owned()),
        _ => HostKind::Unknown,
    }
}

/// `true` se `host` é `base_domain` ou qualquer subdomínio dele (checagem de `Origin`).
pub fn is_within(host: &str, base_domain: &str) -> bool {
    let host = strip_port(host).trim_end_matches('.').to_ascii_lowercase();
    let base = base_domain.trim_end_matches('.').to_ascii_lowercase();
    host == base || host.ends_with(&format!(".{base}"))
}

fn strip_port(host: &str) -> &str {
    match host.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => host,
    }
}

/// Valida um slug de guilda (vira subdomínio).
pub fn validate_slug(slug: &str) -> Result<(), Error> {
    let err = |msg: &str| Err(Error::field("slug", msg));
    if slug.len() < SLUG_MIN || slug.len() > SLUG_MAX {
        return err("deve ter entre 3 e 32 caracteres");
    }
    if !slug
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return err("use apenas letras minúsculas, números e hífen");
    }
    if slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return err("hífen não pode estar no início, no fim nem repetido");
    }
    if RESERVED_SUBDOMAINS.contains(&slug) {
        return err("endereço reservado");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "tiiv.com.br";

    #[test]
    fn classifies_hosts() {
        assert_eq!(classify("tiiv.com.br", BASE), HostKind::Root);
        assert_eq!(classify("www.tiiv.com.br", BASE), HostKind::Root);
        assert_eq!(classify("app.tiiv.com.br", BASE), HostKind::App);
        assert_eq!(classify("ADMIN.tiiv.com.br:443", BASE), HostKind::Admin);
        assert_eq!(
            classify("os-lobos.tiiv.com.br", BASE),
            HostKind::Tenant("os-lobos".into())
        );
        assert_eq!(
            classify("acme.guildos.localhost:5173", "guildos.localhost"),
            HostKind::Tenant("acme".into())
        );
        assert_eq!(classify("api.tiiv.com.br", BASE), HostKind::Unknown);
        assert_eq!(classify("a.b.tiiv.com.br", BASE), HostKind::Unknown);
        assert_eq!(classify("eviltiiv.com.br", BASE), HostKind::Unknown);
        assert_eq!(classify("example.com", BASE), HostKind::Unknown);
    }

    #[test]
    fn origin_check() {
        assert!(is_within("acme.tiiv.com.br", BASE));
        assert!(is_within("tiiv.com.br:443", BASE));
        assert!(!is_within("eviltiiv.com.br", BASE));
        assert!(!is_within("tiiv.com.br.evil.com", BASE));
    }

    #[test]
    fn validates_slugs() {
        assert!(validate_slug("os-lobos").is_ok());
        assert!(validate_slug("abc").is_ok());
        let long = "a".repeat(33);
        for bad in [
            "ab", "-abc", "abc-", "a--b", "Abc", "a_b", "app", "www", &long,
        ] {
            assert!(validate_slug(bad).is_err(), "{bad}");
        }
    }
}
