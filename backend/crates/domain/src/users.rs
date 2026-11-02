use crate::{error::collect, Error};

pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn validate_registration(email: &str, password: &str, display_name: &str) -> Result<(), Error> {
    collect([
        validate_email(email),
        validate_password(password),
        validate_text("display_name", display_name, 2, 40),
    ])
}

pub fn validate_email(email: &str) -> Result<(), Error> {
    let ok = email.len() <= 254
        && !email.chars().any(char::is_whitespace)
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        });
    if ok {
        Ok(())
    } else {
        Err(Error::field("email", "e-mail inválido"))
    }
}

pub fn validate_password(password: &str) -> Result<(), Error> {
    let len = password.chars().count();
    if (PASSWORD_MIN..=PASSWORD_MAX).contains(&len) {
        Ok(())
    } else {
        Err(Error::field(
            "password",
            "a senha deve ter entre 8 e 128 caracteres",
        ))
    }
}

/// Texto obrigatório com tamanho entre `min` e `max` caracteres (após trim).
pub fn validate_text(field: &str, value: &str, min: usize, max: usize) -> Result<(), Error> {
    let len = value.trim().chars().count();
    if (min..=max).contains(&len) {
        Ok(())
    } else {
        Err(Error::field(
            field,
            format!("deve ter entre {min} e {max} caracteres"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_registration() {
        assert!(validate_registration("a@b.co", "12345678", "Bruno").is_ok());
        let Err(Error::Validation(f)) = validate_registration("x", "123", "") else {
            panic!("esperava erro de validação");
        };
        assert_eq!(f.len(), 3);
    }
}
