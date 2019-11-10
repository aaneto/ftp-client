//! Tests for the FTP client crate, all the tests
//! are made with real sample FTP servers.
use ftp_client::error::Error as FtpError;
use ftp_client::prelude::*;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

#[test]
fn name_listing() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;

    assert_eq!(
        vec!["/pub".to_string(), "/readme.txt".to_string()],
        client.list_names("/")?
    );
    Ok(())
}

#[test]
fn pwd() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.cwd("/pub")?;
    let dir = client.pwd()?;
    assert!(dir.contains("/pub"));

    Ok(())
}

#[test]
fn site() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.site_parameters()?;

    Ok(())
}

#[test]
fn file_retrieval() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    let readme_file = client.retrieve_file("/readme.txt")?;
    // Taken previously and unlikely to change
    let file_size = 403;

    assert_eq!(readme_file.len(), file_size);
    Ok(())
}

#[test]
fn cwd() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.cwd("/pub/example")?;

    // The /pub/example dir has many files
    let names = client.list_names("")?;
    assert!(names.len() > 3);

    Ok(())
}

#[test]
fn cdup() -> Result<(), FtpError> {
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
fn logout() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.logout()
}

#[test]
fn noop() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.noop()
}

#[test]
fn help() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    client.help()
}

#[test]
fn store() -> Result<(), FtpError> {
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
fn append() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        let file_data = b"Some data for you";
        let file_name = "/readyou.txt";
        client.append(file_name, file_data)?;

        Ok(())
    });
}

#[test]
fn store_unique() -> Result<(), FtpError> {
    let mut client = Client::connect(
        "speedtest4.tele2.net",
        "anonymous",
        "anonymous@anonymous.com",
    )?;
    client.cwd("/upload/")?;
    let file_data = b"Some data for you";
    client.store_unique(file_data)?;

    Ok(())
}

#[test]
fn system() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    // Should be Windows_NT but we don't need to check that..
    // since we don't want to break tests if the server changes OS
    let _system_name = client.system()?;

    Ok(())
}

#[test]
fn ipv6() -> Result<(), FtpError> {
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
fn tls() -> Result<(), FtpError> {
    let mut client = Client::connect("test.rebex.net", "demo", "password")?;
    // Run random command just to assert we are communicating
    let _system_name = client.system()?;
    Ok(())
}

#[test]
fn rename_file() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.store("testfile", b"DATA")?;
        client.rename_file("testfile", "testfile.txt")?;

        Ok(())
    });
}

#[test]
fn delete_file() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.store("testfile", b"DATA")?;
        client.delete_file("testfile")?;

        Ok(())
    });
}

#[test]
fn create_directory() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.make_directory("new_dir")
    });
}

#[test]
fn delete_directory() {
    run_with_server(|| {
        let mut client = Client::connect("localhost", "user", "user")?;
        client.make_directory("new_dir")?;
        client.remove_directory("new_dir")
    });
}

static SERVER_MUTEX: OnceCell<Mutex<()>> = OnceCell::new();

fn run_with_server<F: Fn() -> Result<(), FtpError>>(func: F) {
    let mutex = SERVER_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex.lock().unwrap();
    // Reset server data
    std::fs::remove_dir_all("res").unwrap();
    std::fs::create_dir("res").unwrap();

    let mut child = std::process::Command::new("python")
        .arg("src/sample_server.py")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let result = func();
    // Clean up before running unwrap on test result
    child.kill().unwrap();
    result.unwrap();
}
