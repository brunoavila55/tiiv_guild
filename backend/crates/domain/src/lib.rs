//! Regras de negócio puras (sem IO). Nada aqui conhece jogo nenhum.

pub mod branding;
pub mod error;
pub mod host;
pub mod members;
pub mod permissions;
pub mod ranks;
pub mod users;

pub use error::{Error, FieldErrors};
