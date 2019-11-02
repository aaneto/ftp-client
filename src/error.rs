use derive_error::Error;
use native_tls::{Error as TlsError, HandshakeError};
use std::net::TcpStream;

#[derive(Debug, Error)]
pub enum Error {
    /// IO Error
    IoError(std::io::Error),
    /// Server responded with failure
    #[error(msg_embedded, no_from, non_std)]
    FailureStatusCode(String),
    /// Unexpected status code
    #[error(msg_embedded, no_from, non_std)]
    UnexpectedStatusCode(String),
    /// A (de)serialization failed
    #[error(msg_embedded, no_from, non_std)]
    SerializationFailed(String),
    /// Invalid socket IP from passive mode
    #[error(msg_embedded, no_from, non_std)]
    InvalidSocketPassiveMode(String),
    /// Error on TLS handshake
    TlsHandshakeError(HandshakeError<TcpStream>),
    /// Error while creating TLS connector
    TlsError(TlsError),
}
