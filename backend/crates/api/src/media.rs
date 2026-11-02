//! Arquivos enviados (logo, banner…): S3/MinIO ou disco local, atrás da mesma interface.
//! No disco local a própria API serve os arquivos em `/api/media/{key}`.

use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use bytes::Bytes;
use object_store::{
    aws::AmazonS3Builder, local::LocalFileSystem, memory::InMemory, path::Path, Attribute,
    Attributes, ObjectStore, PutOptions, PutPayload,
};

pub const LOCAL_PREFIX: &str = "/api/media/";

#[derive(Debug, Clone)]
pub enum StorageConfig {
    Local {
        dir: PathBuf,
    },
    S3 {
        bucket: String,
        endpoint: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        /// URL pública do bucket (ex.: `https://media.tiiv.com.br/guildos`).
        public_url: String,
    },
    /// Só para testes.
    Memory,
}

pub struct Media {
    store: Arc<dyn ObjectStore>,
    /// `None`: a API serve os arquivos (disco local/memória).
    public_base: Option<String>,
}

impl Media {
    pub fn from_config(config: &StorageConfig) -> anyhow::Result<Self> {
        Ok(match config {
            StorageConfig::Local { dir } => {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("criando {}", dir.display()))?;
                Self {
                    store: Arc::new(LocalFileSystem::new_with_prefix(dir)?),
                    public_base: None,
                }
            }
            StorageConfig::S3 {
                bucket,
                endpoint,
                region,
                access_key_id,
                secret_access_key,
                public_url,
            } => Self {
                store: Arc::new(
                    AmazonS3Builder::new()
                        .with_bucket_name(bucket)
                        .with_endpoint(endpoint)
                        .with_region(region)
                        .with_access_key_id(access_key_id)
                        .with_secret_access_key(secret_access_key)
                        .with_allow_http(endpoint.starts_with("http://"))
                        .with_virtual_hosted_style_request(false)
                        .build()?,
                ),
                public_base: Some(public_url.trim_end_matches('/').to_owned()),
            },
            StorageConfig::Memory => Self {
                store: Arc::new(InMemory::new()),
                public_base: None,
            },
        })
    }

    pub fn url(&self, key: &str) -> String {
        match &self.public_base {
            Some(base) => format!("{base}/{key}"),
            None => format!("{LOCAL_PREFIX}{key}"),
        }
    }

    /// Chave de um arquivo nosso a partir da URL salva no banco.
    pub fn key_from_url<'a>(&self, url: &'a str) -> Option<&'a str> {
        match &self.public_base {
            Some(base) => url.strip_prefix(base.as_str())?.strip_prefix('/'),
            None => url.strip_prefix(LOCAL_PREFIX),
        }
    }

    pub fn serves_locally(&self) -> bool {
        self.public_base.is_none()
    }

    pub async fn put(&self, key: &str, bytes: Bytes, mime: &'static str) -> anyhow::Result<()> {
        // Disco local não guarda atributos; ali o content-type vem da extensão ao servir.
        let attributes = if self.public_base.is_some() {
            let mut a = Attributes::new();
            a.insert(Attribute::ContentType, mime.into());
            a.insert(
                Attribute::CacheControl,
                "public, max-age=31536000, immutable".into(),
            );
            a
        } else {
            Attributes::new()
        };
        let opts = PutOptions {
            attributes,
            ..Default::default()
        };
        self.store
            .put_opts(&Path::parse(key)?, PutPayload::from(bytes), opts)
            .await?;
        Ok(())
    }

    /// Remoção best-effort: um arquivo órfão não pode quebrar a requisição.
    pub async fn delete_quietly(&self, key: &str) {
        let result = match Path::parse(key) {
            Ok(path) => self.store.delete(&path).await.map_err(anyhow::Error::from),
            Err(e) => Err(e.into()),
        };
        if let Err(e) = result {
            tracing::warn!(key, error = %e, "não foi possível apagar o arquivo");
        }
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>> {
        let path = Path::parse(key)?;
        match self.store.get(&path).await {
            Ok(r) => Ok(Some(r.bytes().await?)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
