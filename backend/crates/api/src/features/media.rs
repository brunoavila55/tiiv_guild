//! Serve os arquivos enviados quando o storage é local (com S3 o bucket serve direto).

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use domain::{branding::ImageKind, Error};

use crate::{error::ApiResult, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/media/{*key}", get(serve))
}

async fn serve(State(state): State<AppState>, Path(key): Path<String>) -> ApiResult<Response> {
    // Só imagens que nós mesmos gravamos: o tipo vem da extensão da chave gerada no upload.
    let kind = key
        .rsplit_once('.')
        .and_then(|(_, ext)| ImageKind::from_extension(ext))
        .filter(|_| state.media.serves_locally() && key.starts_with("guilds/"))
        .ok_or(Error::NotFound)?;
    let bytes = state.media.get(&key).await?.ok_or(Error::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(kind.mime())),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("default-src 'none'; sandbox"),
            ),
        ],
        bytes,
    )
        .into_response())
}
