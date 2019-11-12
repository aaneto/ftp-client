# ftp-client
![coverage status](https://github.com/aaneto/celtic-names/workflows/Coverage/badge.svg)
![quality gate](https://github.com/aaneto/celtic-names/workflows/Quality%20Gate/badge.svg)
![tests](https://github.com/aaneto/celtic-names/workflows/Tests/badge.svg)

[![codecov](https://codecov.io/gh/aaneto/ftp-client/branch/master/graph/badge.svg)](https://codecov.io/gh/aaneto/ftp-client)
[![Documentation](https://docs.rs/ftp-client/badge.svg)](https://docs.rs/ftp-client)
[![crates.io](https://img.shields.io/crates/v/ftp-client.svg)](https://crates.io/crates/ftp-client)

This crate is my attempt at writting a FTP sync client using Rust, it should contain most commands useful to a regular client with ease of use. Additional internal functionality is also exposed to avoid limiting the user to the current implementation.

Listing the files on the current working directory looks like below when using this crate:

```rust
use ftp_client::prelude::*;

fn main() -> Result<(), ftp_client::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    let names = client.list_names("/")?;
    println!("Listing names: ");
    for name in names {
        println!("{}", name);
    }
    Ok(())
}
```

# Running tests

To run all tests, python3 is needed with the dependency pyftpdlib installed.

This is needed because a python3 ftp server is needed for some tests.
