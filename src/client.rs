use derive_error::Error;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;

#[derive(Debug, PartialEq)]
pub enum StatusCodeKind {
    /// Status code 125
    DataConnectionOpenTransferStarted,
    /// Status code 220
    Ok,
    /// Status code 226
    RequestActionCompleted,
    /// Status code 331
    PasswordRequired,
    /// Status code 230
    UserLoggedIn,
    /// Status code 227
    EnteredPassiveMode,
    Unknown,
}

#[derive(Debug, PartialEq)]
struct StatusCode {
    kind: StatusCodeKind,
    code: u16,
}

impl StatusCode {
    pub fn parse(text: &str) -> Self {
        let code: &u16 = &text[0..3].parse().unwrap();
        let code: u16 = *code;
        let kind = match code {
            125 => StatusCodeKind::DataConnectionOpenTransferStarted,
            220 => StatusCodeKind::Ok,
            226 => StatusCodeKind::RequestActionCompleted,
            331 => StatusCodeKind::PasswordRequired,
            230 => StatusCodeKind::UserLoggedIn,
            227 => StatusCodeKind::EnteredPassiveMode,
            _ => StatusCodeKind::Unknown,
        };

        Self { kind, code }
    }

    pub fn is_failure(&self) -> bool {
        self.code > 399 && self.code < 599
    }
}

#[derive(Debug, PartialEq)]
pub struct ServerResponse {
    message: String,
    status_code: StatusCode,
}

#[derive(Debug, Error)]
pub enum ServerError {
    /// IO Error
    IoError(std::io::Error),
    /// Server responded with failure
    #[error(msg_embedded, no_from, non_std)]
    FailureStatusCode(String),
    /// Unexpected status code
    UnexpectedStatusCode,
}

#[derive(Debug, Clone, Copy)]
pub enum ClientMode {
    Passive,
    Active,
}

impl ServerResponse {
    pub fn parse(text: &str) -> Self {
        let status_code = StatusCode::parse(text);
        let message = text[3..].trim().to_string();

        Self {
            message,
            status_code,
        }
    }

    pub fn is_failure_status(&self) -> bool {
        self.status_code.is_failure()
    }
}

struct Credentials {
    user: String,
    password: String,
}

pub struct ClientBuilder {
    credentials: Option<Credentials>,
    port: u16,
    mode: ClientMode,
}

impl ClientBuilder {
    pub fn new(mode: ClientMode) -> Self {
        Self {
            mode,
            credentials: None,
            port: 21,
        }
    }

    pub fn new_passive() -> Self {
        Self {
            mode: ClientMode::Passive,
            credentials: None,
            port: 21,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_credentials(mut self, user: &str, password: &str) -> Self {
        self.credentials = Some(Credentials {
            user: user.to_owned(),
            password: password.to_owned(),
        });

        self
    }

    pub fn build(&self, hostname: &str) -> Result<Client, ServerError> {
        let mut client = Client::new(hostname)?;

        // Login happens before mode setting
        if let Some(ref creds) = self.credentials {
            client.login(&creds.user, &creds.password)?;
        }
        match self.mode {
            ClientMode::Active => unimplemented!(),
            ClientMode::Passive => client.passive_mode()?,
        };

        Ok(client)
    }
}

pub struct Client {
    stream: BufReader<TcpStream>,
    /// The data stream might be on a remote server
    data_stream: Option<BufReader<TcpStream>>,
    buffer: String,
}

impl Client {
    fn new(hostname: &str) -> Result<Self, ServerError> {
        Self::new_with_port(hostname, 21)
    }

    fn new_with_port(hostname: &str, port: u32) -> Result<Self, ServerError> {
        let host = format!("{}:{}", hostname, port);
        let raw_stream = TcpStream::connect(&host)?;
        let stream = BufReader::new(raw_stream);

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            data_stream: None,
        };
        client.read_reply_expecting(StatusCodeKind::Ok)?;

        Ok(client)
    }

    pub fn login(&mut self, user: &str, password: &str) -> Result<(), ServerError> {
        self.write_unary_command("USER", user)?;
        self.read_reply_expecting(StatusCodeKind::PasswordRequired)?;

        self.write_unary_command("PASS", password)?;
        self.read_reply_expecting(StatusCodeKind::UserLoggedIn)?;

        Ok(())
    }

    pub fn retrieve_file(&mut self, path: &str) -> Result<(), ServerError> {
        self.write_unary_command("RETR", path)?;
        self.read_reply_expecting(StatusCodeKind::DataConnectionOpenTransferStarted)?;

        if let Some(ref mut conn) = self.data_stream {
            let mut buffer = Vec::with_capacity(1024);
            conn.read_to_end(&mut buffer)?;
        }
        self.read_reply_expecting(StatusCodeKind::RequestActionCompleted)?;

        Ok(())
    }

    pub fn passive_mode(&mut self) -> Result<(), ServerError> {
        self.write_command("PASV")?;
        let response = self.read_reply_expecting(StatusCodeKind::EnteredPassiveMode)?;
        let data_connection_socket = self.decode_passive_mode_ip(&response.message);

        if let Some(socket) = data_connection_socket {
            self.data_stream = Some(BufReader::new(TcpStream::connect(socket)?));
        }

        Ok(())
    }

    fn decode_passive_mode_ip(&self, message: &str) -> Option<std::net::SocketAddrV4> {
        let first_bracket = message.find('(');
        let second_bracket = message.find(')');

        match (first_bracket, second_bracket) {
            (Some(start), Some(end)) => {
                // We are dealing with ASCII strings only on this point, so +1 is okay.
                let nums: Vec<u8> = message[start + 1..end]
                    .split(',')
                    .map(|val| val.parse().unwrap())
                    .collect();
                let ip = std::net::Ipv4Addr::new(nums[0], nums[1], nums[2], nums[3]);

                Some(std::net::SocketAddrV4::new(
                    ip,
                    256 * nums[4] as u16 + nums[5] as u16,
                ))
            }
            _ => None,
        }
    }

    fn write_unary_command(&mut self, cmd: &str, arg: &str) -> Result<(), ServerError> {
        let text = format!("{} {}\r\n", cmd, arg);
        self.stream.get_mut().write(text.as_bytes())?;

        Ok(())
    }

    fn write_command(&mut self, cmd: &str) -> Result<(), ServerError> {
        let text = format!("{}\r\n", cmd);
        self.stream.get_mut().write(text.as_bytes())?;

        Ok(())
    }

    fn read_reply_expecting(
        &mut self,
        expected_status_kind: StatusCodeKind,
    ) -> Result<ServerResponse, ServerError> {
        let response = self.read_reply()?;

        if response.status_code.kind == expected_status_kind {
            Ok(response)
        } else if response.is_failure_status() {
            Err(ServerError::FailureStatusCode(response.message))
        } else {
            Err(ServerError::UnexpectedStatusCode)
        }
    }

    fn read_reply(&mut self) -> Result<ServerResponse, ServerError> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer)?;
        Ok(ServerResponse::parse(&self.buffer))
    }
}
