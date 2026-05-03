use crate::backend::BackendError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("backend init failed: {0}")]
    Backend(#[source] BackendError),

    #[error("latency probe failed: {0}")]
    Latency(#[source] BackendError),

    #[error("download failed: {0}")]
    Download(#[source] BackendError),

    #[error("upload failed: {0}")]
    Upload(#[source] BackendError),
}

impl TestError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) => 2,
            Self::Backend(e) | Self::Latency(e) | Self::Download(e) | Self::Upload(e) => {
                match e {
                    BackendError::Network(_) | BackendError::Timeout(_) => 3,
                    BackendError::Protocol(_) | BackendError::NotImplemented => 4,
                    // BackendError is #[non_exhaustive]; future variants default to
                    // protocol-class (contract violation, not transient).
                    #[allow(unreachable_patterns)]
                    _ => 4,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::time::Duration;

    use super::*;
    use crate::backend::BackendError;

    #[test]
    fn test_error_exit_code_mapping() {
        assert_eq!(TestError::Config("bad".to_string()).exit_code(), 2);

        assert_eq!(
            TestError::Backend(BackendError::Timeout(Duration::from_secs(1))).exit_code(),
            3
        );
        assert_eq!(
            TestError::Backend(BackendError::NotImplemented).exit_code(),
            4
        );

        assert_eq!(
            TestError::Latency(BackendError::Timeout(Duration::from_secs(1))).exit_code(),
            3
        );
        assert_eq!(
            TestError::Latency(BackendError::Protocol("p".to_string())).exit_code(),
            4
        );

        assert_eq!(
            TestError::Download(BackendError::Protocol("p".to_string())).exit_code(),
            4
        );

        assert_eq!(
            TestError::Upload(BackendError::NotImplemented).exit_code(),
            4
        );
    }
}
