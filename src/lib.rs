//! This crate is my attempt at writting a FTP sync client
//! using Rust, it should contain most commands useful to
//! a regular client with ease of use. Additional internal
//! functionality is also exposed to avoid limiting the user
//! to the current implementation.
//!
//! Listing the files on the current working directory looks like
//! below when using this crate:
//! ```rust
//! use ftp_client::{error::Error, sync::Client};
//!
//! fn main() -> Result<(), Error> {
//!     let mut client = Client::connect("test.rebex.net", "demo", "password")?;
//!     let names = client.list_names("/")?;
//!     println!("Listing names: ");
//!     for name in names {
//!         println!("{}", name);
//!     }
//!     Ok(())
//! }
//! ```
//!
#![deny(missing_docs)]

pub mod client;
pub mod error;
pub mod status_code;
pub mod sync;

/// The prelude module contains some useful default imports.
pub mod prelude {
    pub use crate::client::Client;
    pub use crate::client::ClientMode;
    pub use crate::status_code::{StatusCode, StatusCodeKind};
}
