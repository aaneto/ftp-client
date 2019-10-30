pub mod client;
pub mod error;
pub mod status_code;

pub mod prelude {
    pub use crate::client::Client;
}

#[cfg(test)]
mod tests;
