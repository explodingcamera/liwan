use std::{future::Future, pin::Pin};

use http::{Request, Response};

/// An HTTP transport used by the client.
pub trait Transport: Send + Sync + 'static {
    /// Sends an HTTP request.
    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<()>, TransportError>> + Send + '_>>;
}

/// An HTTP transport failure.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct TransportError {
    message: String,
}

impl TransportError {
    /// Creates a transport error.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
