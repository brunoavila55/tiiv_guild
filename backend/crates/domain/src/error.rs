#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("recurso não encontrado")]
    NotFound,

    #[error("acesso negado")]
    Forbidden,

    #[error("{0}")]
    Conflict(String),

    #[error("campo `{field}` inválido: {message}")]
    Validation { field: String, message: String },
}
