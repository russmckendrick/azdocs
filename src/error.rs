use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no config file found (tried: {})", paths_tried.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "))]
    NotFound { paths_tried: Vec<PathBuf> },
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("missing required config value `{0}` (set it in azdocs.toml or via {1})")]
    MissingValue(&'static str, &'static str),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("token request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("token endpoint returned HTTP {status}: {detail}")]
    Rejected { status: u16, detail: String },
    #[error("invalid token response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ArgError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("resource graph request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("resource graph returned HTTP {status}: {detail}")]
    Api { status: u16, detail: String },
    #[error("still throttled after {attempts} attempts")]
    ThrottledOut { attempts: u32 },
    #[error("invalid resource graph response: {0}")]
    InvalidResponse(String),
}
