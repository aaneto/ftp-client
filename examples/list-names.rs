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
