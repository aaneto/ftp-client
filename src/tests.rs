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

#[test]
fn test_noop() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.noop()
}

#[test]
fn test_help() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.help()
}

#[test]
fn test_store() -> Result<(), crate::error::Error> {
    let mut client = Client::connect(
        "speedtest4.tele2.net",
        "anonymous",
        "anonymous@anonymous.com",
    )?;
    let file_data = b"Some data for you";
    let file_name = "/upload/readyou.txt";

    client.store(file_name, file_data)
}

#[test]
fn test_system() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    // Should be Windows_NT but we don't need to check that..
    // since we don't want to break tests if the server changes OS
    let _system_name = client.system()?;

    Ok(())
}

#[test]
fn test_ipv6() -> Result<(), crate::error::Error> {
    let mut client = Client::connect(
        "speedtest6.tele2.net",
        "anonymous",
        "anonymous@anonymous.com",
    )?;

    let data = b"DATA";
    let file_path = "/upload/readyou.txt";
    client.store(file_path, data)
}

#[test]
fn test_tls() -> Result<(), crate::error::Error> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    // Run random command just to assert we are communicating
    let _system_name = client.system()?;
    Ok(())
}

#[test]
fn test_rename_file() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.store("testfile", b"DATA")?;
        client.rename_file("testfile", "testfile.txt")?;

        Ok(())
    });
}

#[test]
fn test_delete_file() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.store("testfile", b"DATA")?;
        client.delete_file("testfile")?;

        Ok(())
    });
}

#[test]
fn test_create_directory() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.make_directory("new_dir")
    });
}

#[test]
fn test_delete_directory() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.make_directory("new_dir")?;
        client.remove_directory("new_dir")
    });
}

fn run_with_server<F: Fn() -> Result<(), crate::error::Error>>(func: F) {
    // Reset server data
    std::fs::remove_dir_all("res").unwrap();
    std::fs::create_dir("res").unwrap();

    let mut child = std::process::Command::new("python")
        .arg("src/sample_server.py")
        .spawn()
        .unwrap();
    let result = func();
    // Clean up before running unwrap on test result
    child.kill().unwrap();
    result.unwrap();
}
