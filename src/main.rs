use ftp_client::prelude::*;

fn main() -> Result<(), ServerError> {
    let builder = ClientBuilder::new_passive().with_credentials("demo", "password");
    let mut client = builder.build("test.rebex.net")?;

    client.retrieve_file("readme.txt")?;

    Ok(())
}
