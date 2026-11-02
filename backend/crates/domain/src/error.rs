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

/// Acumula erros de campo enquanto extrai os valores já normalizados.
#[derive(Default)]
pub struct Validator(FieldErrors);

impl Validator {
    /// Valor normalizado, ou o padrão de `T` se inválido (o erro fica acumulado).
    /// Só para validadores de campo, que só devolvem `Error::Validation`.
    pub fn check<T: Default>(&mut self, result: Result<T, Error>) -> T {
        match result {
            Ok(v) => v,
            Err(Error::Validation(f)) => {
                self.0.extend(f);
                T::default()
            }
            Err(other) => {
                self.0.insert("_".into(), other.to_string());
                T::default()
            }
        }
    }

    pub fn finish(self) -> Result<(), Error> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Error::Validation(self.0))
        }
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
