//! Identidade visual da guilda: cores, textos, redes sociais e imagens enviadas.

use std::collections::BTreeMap;

use crate::{error::collect, Error};

pub const MOTTO_MAX: usize = 120;
pub const DESCRIPTION_MAX: usize = 10_000;
pub const SOCIAL_URL_MAX: usize = 200;
pub const IMAGE_MAX_BYTES: usize = 2 * 1024 * 1024;

/// Redes aceitas em `socials` (chave → URL https).
pub const SOCIAL_KEYS: &[&str] = &[
    "discord",
    "website",
    "youtube",
    "twitch",
    "instagram",
    "x",
    "tiktok",
    "facebook",
];

/// Cor no formato `#rrggbb` (normalizada para minúsculas).
pub fn normalize_color(field: &str, value: &str) -> Result<String, Error> {
    let v = value.trim().to_ascii_lowercase();
    let ok = v.len() == 7 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit());
    if ok {
        Ok(v)
    } else {
        Err(Error::field(field, "use uma cor no formato #rrggbb"))
    }
}

/// Valida as redes sociais; descarta valores vazios.
pub fn normalize_socials(
    socials: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    let mut checks = Vec::new();
    for (key, url) in socials {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let field = format!("socials.{key}");
        if !SOCIAL_KEYS.contains(&key.as_str()) {
            checks.push(Err(Error::field(&field, "rede não suportada")));
        } else if !url.starts_with("https://")
            || url.len() > SOCIAL_URL_MAX
            || url.contains(char::is_whitespace)
        {
            checks.push(Err(Error::field(
                &field,
                "use um link https:// de até 200 caracteres",
            )));
        } else {
            out.insert(key.clone(), url.to_owned());
        }
    }
    collect(checks)?;
    Ok(out)
}

/// Texto opcional: vazio vira `None`; acima de `max` caracteres é erro.
pub fn optional_text(
    field: &str,
    value: Option<&str>,
    max: usize,
) -> Result<Option<String>, Error> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(v) if v.chars().count() <= max => Ok(Some(v.to_owned())),
        Some(_) => Err(Error::field(field, format!("máximo de {max} caracteres"))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
}

impl ImageKind {
    /// Detecta pelo conteúdo (assinatura do arquivo), nunca pelo nome ou content-type do cliente.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some(Self::Jpeg)
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "png" => Some(Self::Png),
            "jpg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }
}

/// Valida uma imagem enviada (png/jpg/webp, até 2 MB).
pub fn validate_image(field: &str, bytes: &[u8]) -> Result<ImageKind, Error> {
    if bytes.is_empty() {
        return Err(Error::field(field, "arquivo vazio"));
    }
    if bytes.len() > IMAGE_MAX_BYTES {
        return Err(Error::field(field, "a imagem deve ter no máximo 2 MB"));
    }
    ImageKind::sniff(bytes).ok_or_else(|| Error::field(field, "use uma imagem png, jpg ou webp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(
            normalize_color("c", " #A1B2C3 ").ok().as_deref(),
            Some("#a1b2c3")
        );
        for bad in ["a1b2c3", "#abc", "#gggggg", "#a1b2c3d", "red"] {
            assert!(normalize_color("c", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn socials() {
        let ok = BTreeMap::from([
            ("discord".to_owned(), " https://discord.gg/x ".to_owned()),
            ("youtube".to_owned(), "".to_owned()),
        ]);
        let out = normalize_socials(&ok).ok().unwrap_or_default();
        assert_eq!(out.len(), 1);
        assert_eq!(out["discord"], "https://discord.gg/x");

        let bad = BTreeMap::from([
            ("myspace".to_owned(), "https://x".to_owned()),
            ("x".to_owned(), "javascript:alert(1)".to_owned()),
        ]);
        let Err(Error::Validation(f)) = normalize_socials(&bad) else {
            panic!("esperava erro");
        };
        assert!(f.contains_key("socials.myspace") && f.contains_key("socials.x"));
    }

    #[test]
    fn images() {
        assert_eq!(
            validate_image("f", b"\x89PNG\r\n\x1a\nxxxx").ok(),
            Some(ImageKind::Png)
        );
        assert_eq!(
            validate_image("f", &[0xff, 0xd8, 0xff, 0xe0]).ok(),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(
            validate_image("f", b"RIFF\0\0\0\0WEBPVP8 ").ok(),
            Some(ImageKind::Webp)
        );
        assert!(validate_image("f", b"<svg onload=alert(1)>").is_err());
        assert!(validate_image("f", b"").is_err());
        let mut big = b"\x89PNG\r\n\x1a\n".to_vec();
        big.resize(IMAGE_MAX_BYTES + 1, 0);
        assert!(validate_image("f", &big).is_err());
    }
}
