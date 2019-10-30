//! Tests for the FTP client crate, all the tests
//! are made with real sample FTP servers.

use crate::prelude::*;

#[test]
fn test_name_listing() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;

    assert_eq!(
        vec!["/pub".to_string(), "/readme.txt".to_string()],
        client.list_names("/")?
    );
    Ok(())
}

#[test]
fn test_file_retrieval() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    let readme_file = client.retrieve_file("/readme.txt")?;
    // Taken previously and unlikely to change
    let file_size = 403;

    assert_eq!(readme_file.len(), file_size);
    Ok(())
}

#[test]
fn test_cwd() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.cwd("/pub/example")?;

    // The /pub/example dir has many files
    let names = client.list_names("")?;
    assert!(names.len() > 3);

    Ok(())
}

#[test]
fn test_cdup() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    let initial_names = client.list_names("")?;
    client.cwd("/pub/example")?;

    // Go up two times
    client.cdup()?;
    client.cdup()?;

    let final_names = client.list_names("")?;
    assert_eq!(initial_names, final_names);

    Ok(())
}

#[test]
fn test_logout() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.logout()
}
