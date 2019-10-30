#[derive(Debug, PartialEq)]
pub enum StatusCodeKind {
    /// Status code 125
    TransferStarted,
    /// Status code 150
    TransferAboutToStart,
    /// Status code 200
    Ok,
    /// Status code 211,
    StatusCodeResponse,
    /// Status code 220
    ReadyForNewUser,
    /// Status code 226
    RequestActionCompleted,
    /// Status code 250
    RequestFileActionCompleted,
    /// Status code 331
    PasswordRequired,
    /// Status code 230
    UserLoggedIn,
    /// Status code 227
    EnteredPassiveMode,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub struct StatusCode {
    pub kind: StatusCodeKind,
    pub code: u16,
}

impl StatusCode {
    pub fn parse(text: &str) -> Self {
        let code: &u16 = &text[0..3].parse().unwrap();
        let code: u16 = *code;
        let kind = match code {
            125 => StatusCodeKind::TransferStarted,
            150 => StatusCodeKind::TransferAboutToStart,
            200 => StatusCodeKind::Ok,
            211 => StatusCodeKind::StatusCodeResponse,
            220 => StatusCodeKind::ReadyForNewUser,
            226 => StatusCodeKind::RequestActionCompleted,
            227 => StatusCodeKind::EnteredPassiveMode,
            230 => StatusCodeKind::UserLoggedIn,
            250 => StatusCodeKind::RequestFileActionCompleted,
            331 => StatusCodeKind::PasswordRequired,
            _ => StatusCodeKind::Unknown,
        };

        Self { kind, code }
    }

    pub fn is_failure(&self) -> bool {
        self.code > 399 && self.code < 599
    }
}
