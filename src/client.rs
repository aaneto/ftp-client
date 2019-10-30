use crate::status_code::{StatusCode, StatusCodeKind};
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;

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
    buffer: String,
    welcome_string: Option<String>,
    mode: ClientMode,
}

impl Client {
    pub fn connect(
        hostname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        Self::connect_with_port(hostname, 21, user, password)
    }

    pub fn connect_with_port(
        hostname: &str,
        port: u32,
        user: &str,
        password: &str,
    ) -> Result<Self, crate::error::Error> {
        let host = format!("{}:{}", hostname, port);
        let raw_stream = TcpStream::connect(&host)?;
        let stream = BufReader::new(raw_stream);

        let buffer = String::new();
        let mut client = Client {
            stream,
            buffer,
            welcome_string: None,
            mode: ClientMode::Passive,
        };
        let response = client.read_reply_expecting(vec![StatusCodeKind::ReadyForNewUser])?;
        client.welcome_string = Some(response.message);
        client.login(user, password)?;

        Ok(client)
    }

    pub fn get_welcome(&self) -> Option<&String> {
        self.welcome_string.as_ref()
    }

    pub fn login(&mut self, user: &str, password: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting("USER", user, vec![StatusCodeKind::PasswordRequired])?;
        self.write_unary_command_expecting("PASS", password, vec![StatusCodeKind::UserLoggedIn])?;

        Ok(())
    }

    pub fn cwd(&mut self, dir: &str) -> Result<(), crate::error::Error> {
        self.write_unary_command_expecting(
            "CWD",
            dir,
            vec![StatusCodeKind::RequestFileActionCompleted],
        )?;

        Ok(())
    }

    pub fn cdup(&mut self) -> Result<(), crate::error::Error> {
        self.write_command_expecting("CDUP", vec![StatusCodeKind::RequestFileActionCompleted])?;

        Ok(())
    }

    pub fn status(&mut self) -> Result<String, crate::error::Error> {
        let response =
            self.write_command_expecting("STAT", vec![StatusCodeKind::StatusCodeResponse])?;

        Ok(response.message)
    }

    pub fn list(&mut self, path: &str) -> Result<String, crate::error::Error> {
        match self.mode {
            ClientMode::Passive => {
                let mut conn = self.passive_mode_conn()?;

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
                self.read_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
                let text = String::from_utf8(buffer).map_err(|_| {
                    crate::error::Error::SerializationFailed(
                        "Invalid ASCII returned on server directory listing.".to_string(),
                    )
                })?;
                Ok(text)
            }
            ClientMode::Active => unimplemented!(),
        }
    }

    pub fn list_names(&mut self, path: &str) -> Result<Vec<String>, crate::error::Error> {
        match self.mode {
            ClientMode::Passive => {
                let mut conn = self.passive_mode_conn()?;

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
                self.read_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
                let text = String::from_utf8(buffer).map_err(|_| {
                    crate::error::Error::SerializationFailed(
                        "Invalid ASCII returned on server directory name listing.".to_string(),
                    )
                })?;
                Ok(text.lines().map(|line| line.to_owned()).collect())
            }
            ClientMode::Active => unimplemented!(),
        }
    }

    pub fn retrieve_file(&mut self, path: &str) -> Result<Vec<u8>, crate::error::Error> {
        match self.mode {
            ClientMode::Passive => {
                let mut conn = self.passive_mode_conn()?;

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
                self.read_reply_expecting(vec![StatusCodeKind::RequestActionCompleted])?;
                Ok(buffer)
            }
            ClientMode::Active => unimplemented!(),
        }
    }

    pub fn passive_mode_conn(&mut self) -> Result<BufReader<TcpStream>, crate::error::Error> {
        let response =
            self.write_command_expecting("PASV", vec![StatusCodeKind::EnteredPassiveMode])?;
        let socket = self.decode_passive_mode_ip(&response.message).ok_or(
            crate::error::Error::InvalidSocketPassiveMode(
                "Cannot parse socket sent from server for passive mode.".to_string(),
            ),
        )?;

        Ok(BufReader::new(TcpStream::connect(socket)?))
    }

    pub fn write_unary_command_expecting(
        &mut self,
        cmd: &str,
        arg: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_unary_command(cmd, arg)?;
        self.read_reply_expecting(valid_statuses)
    }

    pub fn write_unary_command(&mut self, cmd: &str, arg: &str) -> Result<(), crate::error::Error> {
        let text = format!("{} {}\r\n", cmd, arg);
        self.stream.get_mut().write(text.as_bytes())?;

        Ok(())
    }

    pub fn write_command_expecting(
        &mut self,
        cmd: &str,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        self.write_command(cmd)?;
        self.read_reply_expecting(valid_statuses)
    }

    pub fn write_command(&mut self, cmd: &str) -> Result<(), crate::error::Error> {
        let text = format!("{}\r\n", cmd);
        self.stream.get_mut().write(text.as_bytes())?;

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

    pub fn read_reply_expecting(
        &mut self,
        valid_statuses: Vec<StatusCodeKind>,
    ) -> Result<ServerResponse, crate::error::Error> {
        let response = self.read_reply()?;
        if valid_statuses.contains(&response.status_code.kind) {
            Ok(response)
        } else if response.is_failure_status() {
            Err(crate::error::Error::FailureStatusCode(response.summarize()))
        } else {
            Err(crate::error::Error::UnexpectedStatusCode(
                response.summarize(),
            ))
        }
    }

    pub fn read_reply(&mut self) -> Result<ServerResponse, crate::error::Error> {
        self.buffer.clear();
        self.stream.read_line(&mut self.buffer)?;
        Ok(ServerResponse::parse(&self.buffer))
    }
}
