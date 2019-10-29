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

impl ServerResponse {
    pub fn summarize(&self) -> String {
        format!("{}: {}", self.status_code.code, self.message)
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    /// IO Error
    IoError(std::io::Error),
    /// Server responded with failure
    #[error(msg_embedded, no_from, non_std)]
    FailureStatusCode(String),
    /// Unexpected status code
    #[error(msg_embedded, no_from, non_std)]
    UnexpectedStatusCode(String),
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

pub struct Client {
    stream: BufReader<TcpStream>,
    /// The data stream might be on a remote server
    data_stream: Option<BufReader<TcpStream>>,
    buffer: String,
    welcome_string: Option<String>,
    mode: ClientMode,
    /// Whether the current mode was set
    mode_set: bool,
}

impl Client {
    pub fn connect(hostname: &str, user: &str, password: &str) -> Result<Self, ServerError> {
        Self::connect_with_port(hostname, 21, user, password)
    }

    pub fn connect_with_port(
        hostname: &str,
        port: u32,
        user: &str,
        password: &str,
    ) -> Result<Self, ServerError> {
        let host = format!("{}:{}", hostname, port);
        let raw_stream = TcpStream::connect(&host)?;
        let stream = BufReader::new(raw_stream);

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            data_stream: None,
            welcome_string: None,
            mode: ClientMode::Passive,
            /// Mode will be set on first command
            mode_set: false,
        };
        let response = client.read_reply_expecting(StatusCodeKind::Ok)?;
        client.welcome_string = Some(response.message);
        client.login(user, password)?;

        Ok(client)
    }

    pub fn get_welcome(&self) -> Option<&String> {
        self.welcome_string.as_ref()
    }

    pub fn login(&mut self, user: &str, password: &str) -> Result<(), ServerError> {
        self.write_unary_command("USER", user)?;
        self.read_reply_expecting(StatusCodeKind::PasswordRequired)?;

        self.write_unary_command("PASS", password)?;
        self.read_reply_expecting(StatusCodeKind::UserLoggedIn)?;

        self.set_mode()?;

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

    fn set_mode(&mut self) -> Result<(), ServerError> {
        if !self.mode_set {
            self.mode_set = true;
            match self.mode {
                ClientMode::Passive => self.passive_mode(),
                ClientMode::Active => unimplemented!(),
            }
        } else {
            Ok(())
        }
    }

    fn read_reply_expecting(
        &mut self,
        expected_status_kind: StatusCodeKind,
    ) -> Result<ServerResponse, ServerError> {
        let response = self.read_reply()?;

        if response.status_code.kind == expected_status_kind {
            Ok(response)
        } else if response.is_failure_status() {
            Err(ServerError::FailureStatusCode(response.summarize()))
        } else {
            Err(ServerError::UnexpectedStatusCode(response.summarize()))
        }
    }

    fn read_reply(&mut self) -> Result<ServerResponse, ServerError> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer)?;
        Ok(ServerResponse::parse(&self.buffer))
    }
}
