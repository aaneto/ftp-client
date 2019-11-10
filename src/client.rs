//! Implement the Client and most actual functionality
//! for it.
//!
//! Most functions were implemented using the RFC959 as reference
//! and may not work as expected with deviant server implementations.
use crate::status_code::{StatusCode, StatusCodeKind};
use derive_more::From;
use log::warn;
use native_tls::{TlsConnector, TlsStream};
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{Read, Write};
use std::net::TcpStream;

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

#[derive(From)]
enum ClientStream {
    TcpStream(TcpStream),
    TlsStream(TlsStream<TcpStream>),
}

impl ClientStream {
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, std::io::Error> {
        match self {
            ClientStream::TcpStream(stream) => stream.peer_addr(),
            ClientStream::TlsStream(stream) => stream.get_ref().peer_addr(),
        }
    }
}

impl Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            ClientStream::TcpStream(stream) => stream.read(buf),
            ClientStream::TlsStream(stream) => stream.read(buf),
        }
    }
}

impl Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self {
            ClientStream::TcpStream(stream) => stream.write(buf),
            ClientStream::TlsStream(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        match self {
            ClientStream::TcpStream(stream) => stream.flush(),
            ClientStream::TlsStream(stream) => stream.flush(),
        }
    }
}

/// The Client is where most of the functionality is, it keeps
/// a control connection open and opens up data connections as
/// commands are issued. This struct is a very thin wrapper over
/// the FTP protocol.
pub struct Client {
    stream: BufReader<ClientStream>,
    buffer: String,
    welcome_string: Option<String>,
    mode: ClientMode,
}

impl Client {
    /// Connect to a new FTP server using plain text (no TLS).
    pub fn connect(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        Self::connect_with_port(hostname, 21, user, password)
    }

    /// Connect to a new FTP server using a secure connection (TLS).
    pub fn connect_tls(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        Self::connect_tls_with_port(hostname, 21, user, password)
    }

    /// Connect to a new FTP server using a secure connection (TLS) on a specific port.
    pub fn connect_tls_with_port(
        hostname: &str,
        port: u32,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let connector = TlsConnector::new()?;

        let host = format!("{}:{}", hostname, port);
        let raw_stream = TcpStream::connect(&host)?;
        let tls_stream = connector.connect(&host, raw_stream)?;
        let stream: BufReader<ClientStream> = BufReader::new(tls_stream.into());

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            welcome_string: None,
            mode: ClientMode::ExtendedPassive,
        };
        let response = client.parse_reply_expecting(vec![StatusCodeKind::ReadyForNewUser])?;
        client.welcome_string = Some(response.message);
        client.login(user, password)?;

        Ok(client)
    }

    /// Connect to a new FTP server using plain text (no TLS) on a specific port.
    pub fn connect_with_port(
        hostname: &str,
        port: u32,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let host = format!("{}:{}", hostname, port);
        let raw_stream = TcpStream::connect(&host)?;
        let stream = BufReader::new(raw_stream.into());

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            welcome_string: None,
            mode: ClientMode::ExtendedPassive,
        };
        let response = client.parse_reply_expecting(vec![StatusCodeKind::ReadyForNewUser])?;
        client.welcome_string = Some(response.message);
        client.login(user, password)?;

        Ok(client)
    }

    /// Get the welcome message sent by the server at the connection establishment.
    pub fn get_welcome(&self) -> Option<&String> {
        self.welcome_string.as_ref()
    }

    /// Login using the given user and password.
    /// Note that many servers require a login with an anonymous user,
    /// such as client.login("anonymous", "anonymous@mail.com").
    pub fn login(&mut self, user: &str, password: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("USER", user, vec![StatusCodeKind::PasswordRequired])?;
        self.write_unary_command_expecting("PASS", password, vec![StatusCodeKind::UserLoggedIn])?;

        Ok(())
    }

    /// Logout from the current user/password pair.
    pub fn logout(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("QUIT", vec![StatusCodeKind::ClosingControlConnection])?;

        Ok(())
    }

    /// Change the working directory on the current session.
    pub fn cwd(&mut self, dir: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "CWD",
            dir,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )?;

        Ok(())
    }

    /// Go up to the parent directory on the current session.
    pub fn cdup(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("CDUP", vec![StatusCodeKind::RequestFileActionCompleted])?;

        Ok(())
    }

    /// Show server information regarding its implementation status
    /// to the user.
    ///
    /// The help command can also be used with an argument to see detailed
    /// information about a single command, this behaviour is not implemented.
    pub fn help(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting(
            "HELP",
            vec![StatusCodeKind::SystemStatus, StatusCodeKind::HelpMessage],
        )?;
        Ok(())
    }

    /// This command should not do anything other than receiving
    /// an OK response from the server.
    pub fn noop(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("NOOP", vec![StatusCodeKind::Ok])?;
        Ok(())
    }

    /// Get the current reported status from the server. This can be used
    /// during transfer and between them. This command can be used with
    /// and argument to get behaviour similar to LIST, this particular
    /// behaviour is not implemented.
    pub fn status(&mut self) -> Result<String, crate::error::Error> {
        let response = self.write_command_expecting("STAT", vec![StatusCodeKind::SystemStatus])?;

        Ok(response.message)
    }

    /// List the provided path in any way the server desires.
    pub fn list(&mut self, path: &str) -> Result<String, crate::error::Error> {
        let mut conn = self.get_data_connection()?;
        self.write_unary_command_expecting(
            "LIST",
            path,
            vec![
                StatusCodeKind::TransferStarted,
                StatusCodeKind::TransferAboutToStart,
            ],
        )?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer)?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
        let text = String::from_utf8(buffer).map_err(|_| {
            crate::error::Error::SerializationFailed(
                "Invalid ASCII returned on server directory listing.".to_string(),
            )
        })?;
        Ok(text)
    }

    /// List the provided path, providing only name information about files and directories.
    pub fn list_names(&mut self, path: &str) -> Result<Vec<String>, crate::error::Error> {
        let mut conn = self.get_data_connection()?;
        self.write_unary_command_expecting(
            "NLST",
            path,
            vec![
                StatusCodeKind::TransferStarted,
                StatusCodeKind::TransferAboutToStart,
            ],
        )?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer)?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
        let text = String::from_utf8(buffer).map_err(|_| {
            crate::error::Error::SerializationFailed(
                "Invalid ASCII returned on server directory name listing.".to_string(),
            )
        })?;
        Ok(text.lines().map(|line| line.to_owned()).collect())
    }

    /// Store a new file on a provided path and name.
    pub fn store<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection()?;
            self.write_unary_command_expecting(
                "STOR",
                path,
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )?;
            conn.get_mut().write_all(data.as_ref())?;
        }

        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;

        Ok(())
    }

    /// Store a new file on a provided path using a random unique name.
    pub fn store_unique<B: AsRef<[u8]>>(&mut self, data: B) -> Result<String, crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection()?;
            self.write_command_expecting(
                "STOU",
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )?;
            conn.get_mut().write_all(data.as_ref())?;
        }

        let reply = self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;

        Ok(reply.message)
    }

    /// Append to a existing file or a create a new one.
    pub fn append<B: AsRef<[u8]>>(
        &mut self,
        path: &str,
        data: B,
    ) -> Result<(), crate::error::Error> {
        // Scope connection so it drops before reading server reply.
        {
            let mut conn = self.get_data_connection()?;
            self.write_unary_command_expecting(
                "APPE",
                path,
                vec![
                    StatusCodeKind::TransferStarted,
                    StatusCodeKind::TransferAboutToStart,
                ],
            )?;
            conn.get_mut().write_all(data.as_ref())?;
        }

        self.parse_reply_expecting(vec![
            StatusCodeKind::RequestActionCompleted,
            StatusCodeKind::RequestFileActionCompleted,
        ])?;

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
    pub fn rename_file(
        &mut self,
        path_from: &str,
        path_to: &str,
    ) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "RNFR",
            path_from,
            vec![StatusCodeKind::RequestActionPending],
        )?;
        self.write_unary_command_expecting(
            "RNTO",
            path_to,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )?;

        Ok(())
    }

    /// Remove an existing directory.
    pub fn remove_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "RMD",
            dir_path,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )?;
        Ok(())
    }

    /// Make a new directory.
    pub fn make_directory(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("MKD", dir_path, vec![StatusCodeKind::PathCreated])?;
        Ok(())
    }

    /// Get the current working directory.
    pub fn pwd(&mut self) -> Result<String, crate::error::Error> {
        let response = self.write_command_expecting("PWD", vec![StatusCodeKind::PathCreated])?;
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
    pub fn site_parameters(&mut self) -> Result<String, crate::error::Error> {
        let response = self.write_command_expecting(
            "SITE",
            vec![StatusCodeKind::Ok, StatusCodeKind::FeatureNotImplemented],
        )?;

        Ok(response.message)
    }

    /// Get the type of operating system on the server.
    pub fn system(&mut self) -> Result<String, crate::error::Error> {
        let response =
            self.write_command_expecting("SYST", vec![StatusCodeKind::NameSystemType])?;

        Ok(response.message)
    }

    /// Delete a file at a path.
    pub fn delete_file(&mut self, dir_path: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "DELE",
            dir_path,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )?;

        Ok(())
    }

    /// Download a file at a path into a byte buffer.
    pub fn retrieve_file(&mut self, path: &str) -> Result<Vec<u8>, crate::error::Error> {
        let mut conn = self.get_data_connection()?;
        self.write_unary_command_expecting(
            "RETR",
            path,
            vec![
                StatusCodeKind::TransferAboutToStart,
                StatusCodeKind::TransferStarted,
            ],
        )?;

        let mut buffer = Vec::with_capacity(1024);
        conn.read_to_end(&mut buffer)?;
        self.parse_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
        Ok(buffer)
    }

    /// Acquire the data connection using the current ClientMode.
    pub fn get_data_connection(&mut self) -> Result<BufReader<TcpStream>, crate::error::Error> {
        match self.mode {
            ClientMode::Active => unimplemented!(),
            ClientMode::Passive => self.passive_mode_connection(),
            ClientMode::ExtendedPassive => self.extended_passive_mode_connection(),
        }
    }

    /// Create a extended passive mode connection.
    pub fn extended_passive_mode_connection(
        &mut self,
    ) -> Result<BufReader<TcpStream>, crate::error::Error> {
        let response =
            self.write_command_expecting("EPSV", vec![StatusCodeKind::EnteredExtendedPassiveMode])?;
        let socket = self.decode_extended_passive_mode_socket(&response.message)?;

        Ok(BufReader::new(TcpStream::connect(socket)?))
    }

    /// Create a passive mode connection.
    pub fn passive_mode_connection(&mut self) -> Result<BufReader<TcpStream>, crate::error::Error> {
        let response =
            self.write_command_expecting("PASV", vec![StatusCodeKind::EnteredPassiveMode])?;
        let socket = self.decode_passive_mode_ip(&response.message)?;

        Ok(BufReader::new(TcpStream::connect(socket)?))
    }

    /// Write a command with one argument to the server expecting a list of positive status codes.
    pub fn write_unary_command_expecting(
        &mut self,
        cmd: &str,
        arg: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_unary_command(cmd, arg)?;
        self.parse_reply_expecting(valid_statuses)
    }

    /// Write a command with one argument to the server.
    pub fn write_unary_command(&mut self, cmd: &str, arg: &str) -> Result<(), crate::error::Error> {
        let text = format!("{} {}\r\n", cmd, arg);
        self.stream.get_mut().write_all(text.as_bytes())?;

        Ok(())
    }

    /// Write a command to the server expecting a list of positive status codes.
    pub fn write_command_expecting(
        &mut self,
        cmd: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_command(cmd)?;
        self.parse_reply_expecting(valid_statuses)
    }

    /// Write a command to the server.
    pub fn write_command(&mut self, cmd: &str) -> Result<(), crate::error::Error> {
        let text = format!("{}\r\n", cmd);
        self.stream.get_mut().write_all(text.as_bytes())?;

        Ok(())
    }

    /// Parse the server reply into a ServerResponse expecting a list of status codes.
    pub fn parse_reply_expecting(
        &mut self,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        let response = self.parse_reply()?;

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
    pub fn parse_reply(&mut self) -> Result<ServerResponse, crate::error::Error> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer)?;
        Ok(ServerResponse::parse(&self.buffer))
    }

    /// Read the server reply as a raw string.
    pub fn read_reply(&mut self) -> Result<String, crate::error::Error> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer)?;
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
