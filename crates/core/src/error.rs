use thiserror::Error;

#[derive(Debug, Error)]
pub enum BmsgError {
    #[error("message not found")]
    NotFound,

    #[error("service not found")]
    ServiceNotFound,

    #[error("unauthorized: invalid secret")]
    Unauthorized,

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("delivery error: {0}")]
    DeliveryError(String),

    #[error("election error: {0}")]
    ElectionError(String),

    #[error("registry error: {0}")]
    RegistryError(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// 业务错误码（4位）
impl BmsgError {
    pub fn code(&self) -> u16 {
        match self {
            Self::NotFound => 1004,
            Self::ServiceNotFound => 1004,
            Self::Unauthorized => 1002,
            Self::InvalidRequest(_) => 1003,
            Self::StorageError(_)
            | Self::DeliveryError(_)
            | Self::ElectionError(_)
            | Self::RegistryError(_)
            | Self::Internal(_) => 1000,
        }
    }
}
