//! Regras de conta de usuário.

use crate::DomainError;

pub const PASSWORD_MIN_LEN: usize = 8;
pub const PASSWORD_MAX_LEN: usize = 128;
pub const DISPLAY_NAME_MAX_LEN: usize = 64;
const EMAIL_MAX_LEN: usize = 254;

/// Dados de cadastro já validados e normalizados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    pub email: String,
    pub display_name: String,
}

pub fn validate_registration(
    email: &str,
    password: &str,
    display_name: &str,
) -> Result<NewUser, DomainError> {
    let email = normalize_email(email)?;
    validate_password(password)?;

    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(invalid("display_name", "obrigatório"));
    }
    if display_name.chars().count() > DISPLAY_NAME_MAX_LEN {
        return Err(invalid(
            "display_name",
            &format!("máximo de {DISPLAY_NAME_MAX_LEN} caracteres"),
        ));
    }

    Ok(NewUser {
        email,
        display_name: display_name.to_owned(),
    })
}

/// Remove espaços e checa o formato básico (`algo@dominio.tld`).
pub fn normalize_email(email: &str) -> Result<String, DomainError> {
    let email = email.trim();
    let valid = email.len() <= EMAIL_MAX_LEN
        && !email.chars().any(char::is_whitespace)
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
        });
    if valid {
        Ok(email.to_owned())
    } else {
        Err(invalid("email", "e-mail inválido"))
    }
}

fn validate_password(password: &str) -> Result<(), DomainError> {
    let len = password.chars().count();
    if len < PASSWORD_MIN_LEN {
        return Err(invalid(
            "password",
            &format!("mínimo de {PASSWORD_MIN_LEN} caracteres"),
        ));
    }
    if len > PASSWORD_MAX_LEN {
        return Err(invalid(
            "password",
            &format!("máximo de {PASSWORD_MAX_LEN} caracteres"),
        ));
    }
    Ok(())
}

fn invalid(field: &str, message: &str) -> DomainError {
    DomainError::Validation {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn field_of(err: DomainError) -> String {
        match err {
            DomainError::Validation { field, .. } => field,
            other => panic!("esperava Validation, veio {other:?}"),
        }
    }

    #[test]
    fn accepts_and_normalizes_valid_input() {
        let user = validate_registration("  Lider@Guilda.com ", "senha-forte", "  Líder ").unwrap();
        assert_eq!(user.email, "Lider@Guilda.com");
        assert_eq!(user.display_name, "Líder");
    }

    #[test]
    fn rejects_bad_email() {
        for email in [
            "",
            "sem-arroba",
            "@guilda.com",
            "a@semponto",
            "a@.com",
            "a b@c.com",
            "a@b@c.com",
        ] {
            let err = validate_registration(email, "senha-forte", "Líder").unwrap_err();
            assert_eq!(field_of(err), "email", "{email}");
        }
    }

    #[test]
    fn rejects_short_or_huge_password() {
        let err = validate_registration("a@b.com", "curta", "Líder").unwrap_err();
        assert_eq!(field_of(err), "password");
        let err = validate_registration("a@b.com", &"x".repeat(129), "Líder").unwrap_err();
        assert_eq!(field_of(err), "password");
    }

    #[test]
    fn rejects_blank_or_long_display_name() {
        let err = validate_registration("a@b.com", "senha-forte", "   ").unwrap_err();
        assert_eq!(field_of(err), "display_name");
        let err = validate_registration("a@b.com", "senha-forte", &"x".repeat(65)).unwrap_err();
        assert_eq!(field_of(err), "display_name");
    }
}
