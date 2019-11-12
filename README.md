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

To run all tests a few dependencies are needed, you need to run the docker image contained in "sample-server", like below:

### Building the image
```bash
cd sample-server
docker build . -t ftp-server
```

### Running the image on the background

```bash
docker run -d -p 20:20 -p 21:21 -p 2558:2558 -p 2559:2559 ftp-server
```

After that, you can run ```cargo test``` as you normally would.