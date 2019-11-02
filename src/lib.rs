pub mod client;
pub mod error;
pub mod status_code;

pub mod prelude {
    pub use crate::client::Client;
    pub use crate::status_code::{StatusCode, StatusCodeKind};
}

#[cfg(test)]
mod tests;
