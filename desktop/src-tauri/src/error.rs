use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database path is not a file: {0}")]
    InvalidDatabase(String),
    #[error("application state is unavailable: {0}")]
    State(String),
    #[error("snapshot store error: {0}")]
    Store(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("collection error: {0}")]
    Collection(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<azdocs::error::StoreError> for AppError {
    fn from(error: azdocs::error::StoreError) -> Self {
        Self::Store(error.to_string())
    }
}
