//! Implement the Client and most actual functionality
//! for it.
//!
//! Most functions were implemented using the RFC959 as reference
//! and may not work as expected with deviant server implementations.
use crate::status_code::{StatusCode, StatusCodeKind};
use log::warn;
use std::net::ToSocketAddrs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Represents a raw server response, with
/// a status code and the message after it.
///
/// Note that usual FTP responses follow the format:
/// STATUS_CODE: MESSAGE.
#[derive(Debug, PartialEq)]
pub struct ServerResponse {
    message: String,
    status_code: StatusCode,
}

impl ServerResponse {
    /// Summarize an error message, when a response is not one of the expect
    /// status codes provided.
    pub fn summarize_error(&self, expected: Vec<StatusCodeKind>) -> String {
        format!(
            "Got {}: {}, expected {:?}",
            self.status_code.code, self.message, expected
        )
    }
}

/// A FTP client can run in two main modes: active and passive.
///
/// The passive mode is the simpler to run, you simply ask the
/// server for a host to connect and connect to it, this is your
/// data connection (for more information about the FTP protocol, check
/// RFC959).
///
/// The active mode is different, you must open a port on your machine
/// and use that to connect to the server. This can be a problem if you
/// have firewalls set on your machine/network, for the common user, the
/// passive mode should work fine.
///
/// The ExtendedPassive mode listed is simply the passive mode with support for
/// IPV6.
#[derive(Debug, Clone, Copy)]
pub enum ClientMode {
    /// The passive mode, using the PASV command
    Passive,
    /// The extended passive mode, using the EPSV command
    ExtendedPassive,
    /// The active mode, not implemented yet
    Active,
}

impl ServerResponse {
    /// Parse a server response from the server text response.
    pub fn parse(text: &str) -> Self {
        let status_code = StatusCode::parse(text);
        let message = text[3..].trim().to_string();

        Self {
            message,
            status_code,
        }
    }

    /// Returns whether the status code returned indicates failure.
    pub fn is_failure_status(&self) -> bool {
        self.status_code.is_failure()
    }
}

/// The Client is where most of the functionality is, it keeps
/// a control connection open and opens up data connections as
/// commands are issued. This struct is a very thin wrapper over
/// the FTP protocol.
pub struct Client {
    stream: BufReader<TcpStream>,
    buffer: String,
    welcome_string: Option<String>,
    mode: ClientMode,
}

impl Client {
    /// Set the mode for the client.
    pub fn set_mode(&mut self, mode: ClientMode) {
        self.mode = mode
    }

    /// Connect to a new FTP server using plain text (no TLS).
    pub async fn connect(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        Self::connect_with_port(hostname, 21, user, password).await
    }

    /// Connect to a new FTP server using plain text (no TLS) on a specific port.
    pub async fn connect_with_port(
        hostname: &str,
        port: u32,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let host = format!("{}:{}", hostname, port);
        let addr = host.to_socket_addrs()?.next().unwrap();
        let raw_stream = TcpStream::connect(&addr).await?;
        let stream = BufReader::new(raw_stream);

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            welcome_string: None,
            mode: ClientMode::ExtendedPassive,
        };
        let response = client
            .parse_reply_expecting(vec![StatusCodeKind::ReadyForNewUser])
            .await?;
        client.welcome_string = Some(response.message);
        client.login(user, password).await?;

        Ok(client)
    }

    /// Get the welcome message sent by the server at the connection establishment.
    pub fn get_welcome(&self) -> Option<&String> {
        self.welcome_string.as_ref()
    }

    /// Login using the given user and password.
    /// Note that many servers require a login with an anonymous user,
    /// such as client.login("anonymous", "anonymous@mail.com").
    pub async fn login(&mut self, user: &str, password: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("USER", user, vec![StatusCodeKind::PasswordRequired])
            .await?;
        self.write_unary_command_expecting("PASS", password, vec![StatusCodeKind::UserLoggedIn])
            .await?;

        Ok(())
    }

    /// Logout from the current user/password pair.
    pub async fn logout(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("QUIT", vec![StatusCodeKind::ClosingControlConnection])
            .await?;

        Ok(())
    }

    /// Change the working directory on the current session.
    pub async fn cwd(&mut self, dir: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "CWD",
            dir,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )
        .await?;

        Ok(())
    }

    /// Go up to the parent directory on the current session.
    pub async fn cdup(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("CDUP", vec![StatusCodeKind::RequestFileActionCompleted])
            .await?;

        Ok(())
    }

    /// Show server information regarding its implementation status
    /// to the user.
    ///
    /// The help command can also be used with an argument to see detailed
    /// information about a single command, this behaviour is not implemented.
    pub async fn help(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting(
            "HELP",
            vec![StatusCodeKind::SystemStatus, StatusCodeKind::HelpMessage],
        )
        .await?;
        Ok(())
    }

    /// This command should not do anything other than receiving
    /// an OK response from the server.
    pub async fn noop(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("NOOP", vec![StatusCodeKind::Ok])
            .await?;
        Ok(())
    }

    /// Set the transfer type to ascii
    pub async fn ascii(&mut self) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("TYPE", "A", vec![StatusCodeKind::Ok])
            .await?;
        Ok(())
    }

    /// Set the transfer type to binary
    pub async fn binary(&mut self) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("TYPE", "I", vec![StatusCodeKind::Ok])
            .await?;
        Ok(())
    }

    /// Get the current reported status from the server. This can be used
    /// during transfer and between them. This command can be used with
    /// and argument to get behaviour similar to LIST, this particular
    /// behaviour is not implemented.
    pub async fn status(&mut self) -> Result<String, crate::error::Error> {
        let response = self
            .write_command_expecting("STAT", vec![StatusCodeKind::SystemStatus])
            .await?;

        Ok(response.message)
    }

    /// List the provided path in any way the server desires.
    pub async fn list(&mut self, path: &str) -> Result<String, crate::error::Error> {
        let mut conn = self.get_data_connection().await?;
        self.write_unary_command_expecting(
            "LIST",
            path,
            vec![
                StatusCodeKind::TransferStarted,
                StatusCodeKind::TransferAboutToStart,
            ],
        )
        .await?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer).await?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])
            .await?;
        let text = String::from_utf8(buffer).map_err(|_| {
            crate::error::Error::SerializationFailed(
                "Invalid ASCII returned on server directory listing.".to_string(),
            )
        })?;
        Ok(text)
    }

    /// List the provided path, providing only name information about files and directories.
    pub async fn list_names(&mut self, path: &str) -> Result<Vec<String>, crate::error::Error> {
        let mut conn = self.get_data_connection().await?;
        self.write_unary_command_expecting(
            "NLST",
            path,
            vec![
                StatusCodeKind::TransferStarted,
                StatusCodeKind::TransferAboutToStart,
            ],
        )
        .await?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer).await?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])
            .await?;
        let text = String::from_utf8(buffer).map_err(|_| {
            crate::error::Error::SerializationFailed(
                "Invalid ASCII returned on server directory name listing.".to_string(),
            )
        })?;
        Ok(text.lines().map(|line| line.to_owned()).collect())
    }

    /// Store a new file on a provided path and name.
    pub async fn store<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection().await?;
            self.write_unary_command_expecting(
                "STOR",
                path,
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )
            .await?;
            conn.write_all(data.as_ref()).await?;
        }

        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])
            .await?;

        Ok(())
    }

    /// Store a new file on a provided path using a random unique name.
    pub async fn store_unique<B: AsRef<[u8]>>(
        &mut self,
        data: B,
    ) -> Result<String, crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection().await?;
            self.write_command_expecting(
                "STOU",
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )
            .await?;
            conn.write_all(data.as_ref()).await?;
        }

        let reply = self
            .parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])
            .await?;

        Ok(reply.message)
    }

    /// Append to a existing file or a create a new one.
    pub async fn append<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection().await?;
            self.write_unary_command_expecting(
                "APPE",
                path,
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )
            .await?;
            conn.write_all(data.as_ref()).await?;
        }

        self.parse_reply_expecting(vec![
            StatusCodeKind::RequestActionCompleted,
            StatusCodeKind::RequestFileActionCompleted,
        ])
        .await?;

        Ok(())
    }

    /// Restart a file transfer. Unimplemented.
    pub fn restart(&mut self) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Abort a file transfer. Unimplemented.
    pub fn abort(&mut self) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Preallocate space on the server. Unimplemented.
    pub fn allocate(
        &mut self,
        _logical_size: usize,
        _logical_page_size: Option<usize>,
    ) -> Result<(), crate::error::Error> {
        unimplemented!();
    }

    /// Move a file from a path to another, essentially renaming it.
    pub async fn rename_file(
        &mut self,
        path_from: &str,
        path_to: &str,
    ) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "RNFR",
            path_from,
            vec![StatusCodeKind::RequestActionPending],
        )
        .await?;
        self.write_unary_command_expecting(
            "RNTO",
            path_to,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )
        .await?;

        Ok(())
    }

    /// Remove an existing directory.
    pub async fn remove_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "RMD",
            dir_path,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )
        .await?;
        Ok(())
    }

    /// Make a new directory.
    pub async fn make_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("MKD", dir_path, vec![StatusCodeKind::PathCreated])
            .await?;
        Ok(())
    }

    /// Get the current working directory.
    pub async fn pwd(&mut self) -> Result<String, crate::error::Error> {
        let response = self
            .write_command_expecting("PWD", vec![StatusCodeKind::PathCreated])
            .await?;
        Ok(response.message)
    }

    /// This command is used by the server to provide services
    /// specific to his system that are essential to file transfer
    /// but not sufficiently universal to be included as commands in
    /// the protocol.
    ///
    /// The nature of these services and the
    /// specification of their syntax can be stated in a reply to
    /// the HELP SITE command.
    ///
    /// Extracted from RFC959.
    pub async fn site_parameters(&mut self) -> Result<String, crate::error::Error> {
        let response = self
            .write_command_expecting(
                "SITE",
                vec![StatusCodeKind::Ok, StatusCodeKind::FeatureNotImplemented],
            )
            .await?;

        Ok(response.message)
    }

    /// Get the type of operating system on the server.
    pub async fn system(&mut self) -> Result<String, crate::error::Error> {
        let response = self
            .write_command_expecting("SYST", vec![StatusCodeKind::NameSystemType])
            .await?;

        Ok(response.message)
    }

    /// Delete a file at a path.
    pub async fn delete_file(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "DELE",
            dir_path,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )
        .await?;

        Ok(())
    }

    /// Download a file at a path into a byte buffer.
    pub async fn retrieve_file(&mut self, path: &str) -> Result<Vec<u8>, crate::error::Error> {
        let mut conn = self.get_data_connection().await?;
        self.write_unary_command_expecting(
            "RETR",
            path,
            vec![
                StatusCodeKind::TransferAboutToStart,
                StatusCodeKind::TransferStarted,
            ],
        )
        .await?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer).await?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])
            .await?;
        Ok(buffer)
    }

    /// Acquire the data connection using the current ClientMode.
    pub async fn get_data_connection(&mut self) -> Result<TcpStream, crate::error::Error> {
        match self.mode {
            ClientMode::Active => unimplemented!(),
            ClientMode::Passive => self.passive_mode_connection().await,
            ClientMode::ExtendedPassive => self.extended_passive_mode_connection().await,
        }
    }

    /// Create a extended passive mode connection.
    pub async fn extended_passive_mode_connection(
        &mut self,
    ) -> Result<TcpStream, crate::error::Error> {
        let response = self
            .write_command_expecting("EPSV", vec![StatusCodeKind::EnteredExtendedPassiveMode])
            .await?;
        let socket = self.decode_extended_passive_mode_socket(&response.message)?;

        Ok(TcpStream::connect(socket).await?)
    }

    /// Create a passive mode connection.
    pub async fn passive_mode_connection(&mut self) -> Result<TcpStream, crate::error::Error> {
        let response = self
            .write_command_expecting("PASV", vec![StatusCodeKind::EnteredPassiveMode])
            .await?;
        let socket = self.decode_passive_mode_ip(&response.message)?;

        Ok(TcpStream::connect(socket).await?)
    }

    /// Write a command with one argument to the server expecting a list of positive status codes.
    pub async fn write_unary_command_expecting(
        &mut self,
        cmd: &str,
        arg: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_unary_command(cmd, arg).await?;
        self.parse_reply_expecting(valid_statuses).await
    }

    /// Write a command with one argument to the server.
    pub async fn write_unary_command(
        &mut self,
        cmd: &str,
        arg: &str,
    ) -> Result<(), crate::error::Error> {
        let text = format!("{} {}\r\n", cmd, arg);
        self.stream.get_mut().write_all(text.as_bytes()).await?;

        Ok(())
    }

    /// Write a command to the server expecting a list of positive status codes.
    pub async fn write_command_expecting(
        &mut self,
        cmd: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_command(cmd).await?;
        self.parse_reply_expecting(valid_statuses).await
    }

    /// Write a command to the server.
    pub async fn write_command(&mut self, cmd: &str) -> Result<(), crate::error::Error> {
        let text = format!("{}\r\n", cmd);
        self.stream.get_mut().write_all(text.as_bytes()).await?;

        Ok(())
    }

    /// Parse the server reply into a ServerResponse expecting a list of status codes.
    pub async fn parse_reply_expecting(
        &mut self,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        let response = self.parse_reply().await?;

        let is_expected_status = valid_statuses.contains(&response.status_code.kind);
        // We are a bit liberal on what we accept.
        let is_positive_status = response.status_code.is_valid();
        warn!(
            "Unexpected positive status was accepted: {:?}",
            response.status_code
        );

        if is_expected_status || is_positive_status {
            Ok(response)
        } else {
            Err(crate::error::Error::UnexpectedStatusCode(
                response.summarize_error(valid_statuses),
            ))
        }
    }

    /// Parse the server reply into a ServerResponse.
    pub async fn parse_reply(&mut self) -> Result<ServerResponse, crate::error::Error> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer).await?;
        Ok(ServerResponse::parse(&self.buffer))
    }

    /// Read the server reply as a raw string.
    pub async fn read_reply(&mut self) -> Result<String, crate::error::Error> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer).await?;
        Ok(self.buffer.clone())
    }

    fn decode_passive_mode_ip(
        &self,
        message: &str,
    ) -> Result<std::net::SocketAddrV4, crate::error::Error> {
        let first_bracket = message.find('(');
        let second_bracket = message.find(')');
        let cant_parse_error = || {
            crate::error::Error::InvalidSocketPassiveMode(format!(
                "Cannot parse socket sent from server for passive mode: {}.",
                message
            ))
        };

        match (first_bracket, second_bracket) {
            (Some(start), Some(end)) => {
                // We are dealing with ASCII strings only on this point, so +1 is okay.
                let nums: Vec<u8> = message[start + 1..end]
                    .split(',')
                    // Try to parse all digits between ','
                    .flat_map(|val| val.parse())
                    .collect();
                if nums.len() < 4 {
                    Err(cant_parse_error())
                } else {
                    let ip = std::net::Ipv4Addr::new(nums[0], nums[1], nums[2], nums[3]);

                    Ok(std::net::SocketAddrV4::new(
                        ip,
                        256 * nums[4] as u16 + nums[5] as u16,
                    ))
                }
            }
            _ => Err(cant_parse_error()),
        }
    }

    fn decode_extended_passive_mode_socket(
        &self,
        response: &str,
    ) -> Result<std::net::SocketAddr, crate::error::Error> {
        let first_delimiter = response.find("|||");
        let second_delimiter = response.rfind('|');
        let cant_parse_error = || {
            crate::error::Error::InvalidSocketPassiveMode(format!(
                "Cannot parse socket sent from server for passive mode: {}.",
                response
            ))
        };

        match (first_delimiter, second_delimiter) {
            (Some(start), Some(end)) => {
                let port: u16 = response[start + 3..end]
                    .parse()
                    .map_err(move |_| cant_parse_error())?;
                let ip = self
                    .stream
                    .get_ref()
                    .peer_addr()
                    .map_err(move |_| cant_parse_error())?
                    .ip();
                Ok(std::net::SocketAddr::new(ip, port))
            }
            _ => Err(cant_parse_error()),
        }
    }
}
