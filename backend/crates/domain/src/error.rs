use std::collections::BTreeMap;

/// Erros por campo de um payload inválido (campo → mensagem).
pub type FieldErrors = BTreeMap<String, String>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("dados inválidos")]
    Validation(FieldErrors),
    #[error("{0}")]
    Conflict(&'static str),
    #[error("não encontrado")]
    NotFound,
    #[error("autenticação necessária")]
    Unauthenticated,
    #[error("sem permissão")]
    Forbidden,
}

impl Error {
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        Self::Validation(FieldErrors::from([(field.to_owned(), message.into())]))
    }
}

/// Junta erros de vários campos num único `Error::Validation`.
pub fn collect(results: impl IntoIterator<Item = Result<(), Error>>) -> Result<(), Error> {
    let mut fields = FieldErrors::new();
    for r in results {
        match r {
            Ok(()) => {}
            Err(Error::Validation(f)) => fields.extend(f),
            Err(other) => return Err(other),
        }
    }
    if fields.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation(fields))
    }
}
