//! Tests for the FTP client crate, all the tests
//! are made with real sample FTP servers.
//!
//! Tests that start with test_ are run with
//! external FTP servers, the others are run
//! with a local dockerize server that you should start.
use ftp_client::error::Error as FtpError;
use ftp_client::sync::Client as SyncClient;
use once_cell::sync::OnceCell;
use std::io::Read;
use std::sync::Mutex;

#[test]
fn test_name_listing() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    assert_eq!(
        vec!["example".to_string(), "sample.txt".to_string(),],
        client.list_names("/pub/")?
    );
    Ok(())
}

#[test]
fn test_pwd() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.cwd("/pub")?;
    let dir = client.pwd()?;
    assert!(dir.contains("/pub"));

    Ok(())
}

#[test]
fn test_site() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.site_parameters(Some("HELP".to_string()))?;

    Ok(())
}

#[test]
fn test_file_retrieval() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    let cat_file = client.retrieve_file("/cat.png")?;
    // Taken previously and unlikely to change
    let file_size = 29712;

    assert_eq!(cat_file.len(), file_size);
    Ok(())
}

#[test]
fn test_cwd() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.cwd("/pub/example")?;

    // The /pub/example dir has many files
    let names = client.list_names("")?;
    dbg!(names.clone());
    assert!(names.len() > 3);

    Ok(())
}

#[test]
fn test_cdup() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
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
fn test_logout() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.logout()
}

#[test]
fn test_noop() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.noop()
}

#[test]
fn test_help() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    client.help(Some("LIST".to_string()))
}

#[test]
fn test_store() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    let file_data = b"Some data for you";
    let file_name = "/pub/example/readyou.txt";

    client.store(file_name, file_data)
}

#[test]
fn test_store_unique() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;

    client.cwd("/pub/example/")?;
    let file_data = b"Some data for you";
    client.store_unique(file_data)?;

    Ok(())
}

#[test]
fn test_system() -> Result<(), FtpError> {
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    // Should be Windows_NT but we don't need to check that..
    // since we don't want to break tests if the server changes OS
    let _system_name = client.system()?;

    Ok(())
}

#[test]
fn append() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    let file_data = b"Some data for you";
    let file_name = "readyou.txt";
    client.append(file_name, file_data)?;

    Ok(())
}

#[test]
fn rename_file() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    if !client.list_names("/")?.contains(&"testfile".to_string()) {
        client.store("/testfile", b"DATA")?;
    }
    client.rename_file("/testfile", "/testfile.txt")?;

    Ok(())
}

#[test]
fn delete_file() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    if !client.list_names("/")?.contains(&"testfile".to_string()) {
        client.store("testfile", b"DATA")?;
    }
    client.delete_file("testfile")?;

    Ok(())
}

#[test]
fn create_directory() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    if client.list_names("/")?.contains(&"new_dir".to_string()) {
        client.remove_directory("new_dir")?;
    }
    client.make_directory("new_dir")
}

#[test]
fn delete_directory() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;
    if !client.list_names("/")?.contains(&"new_dir".to_string()) {
        client.make_directory("new_dir")?;
    }
    client.remove_directory("new_dir")
}

#[test]
fn binary_transfer() -> Result<(), FtpError> {
    lock_server();
    let mut client = SyncClient::connect(&get_local_server_hostname(), "user", "user")?;

    let file_bytes_ascii = client.retrieve_file("cat.png")?;
    client.binary()?;
    let file_bytes_binary = client.retrieve_file("cat.png")?;

    let mut reference_file = std::fs::File::open("res/cat.png").unwrap();
    let mut reference_bytes = Vec::new();
    reference_file.read_to_end(&mut reference_bytes).unwrap();

    assert!(file_bytes_ascii != reference_bytes);
    assert_eq!(file_bytes_binary, reference_bytes);

    Ok(())
}

/// Get the hostname for the local server.
fn get_local_server_hostname() -> String {
    std::env::var("SERVER_HOSTNAME").expect("SERVER_HOSTNAME is not set.")
}

static SERVER_MUTEX: OnceCell<Mutex<()>> = OnceCell::new();
/// Tests using the local server can not run concurrently.
fn lock_server() {
    let mutex = SERVER_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex.lock().expect("Could not lock server.");
    std::thread::sleep(std::time::Duration::from_millis(500));
}
