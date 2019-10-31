#[derive(Debug, PartialEq)]
pub enum StatusCodeKind {
    /// Status code 125
    TransferStarted,
    /// Status code 150
    TransferAboutToStart,
    /// Status code 200
    Ok,
    /// Status code 202
    FeatureNotImplemented,
    /// Status code 211,
    SystemStatus,
    /// Status code 214
    HelpMessage,
    /// Status code 215
    NameSystemType,
    /// Status code 220
    ReadyForNewUser,
    /// Status code 221
    ClosingControlConnection,
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
    /// Status code 550
    RequestActionDenied,
    Unknown,
}

impl From<u16> for StatusCodeKind {
    fn from(code: u16) -> StatusCodeKind {
        match code {
            125 => StatusCodeKind::TransferStarted,
            150 => StatusCodeKind::TransferAboutToStart,
            200 => StatusCodeKind::Ok,
            202 => StatusCodeKind::FeatureNotImplemented,
            211 => StatusCodeKind::SystemStatus,
            214 => StatusCodeKind::HelpMessage,
            215 => StatusCodeKind::NameSystemType,
            221 => StatusCodeKind::ClosingControlConnection,
            220 => StatusCodeKind::ReadyForNewUser,
            226 => StatusCodeKind::RequestActionCompleted,
            227 => StatusCodeKind::EnteredPassiveMode,
            230 => StatusCodeKind::UserLoggedIn,
            250 => StatusCodeKind::RequestFileActionCompleted,
            331 => StatusCodeKind::PasswordRequired,
            550 => StatusCodeKind::RequestActionDenied,
            _ => StatusCodeKind::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct StatusCode {
    pub kind: StatusCodeKind,
    pub code: u16,
}

impl PartialEq for StatusCode {
    fn eq(&self, other: &StatusCode) -> bool {
        self.code == other.code
    }
}

impl StatusCode {
    pub fn parse(text: &str) -> Self {
        let code: &u16 = &text[0..3].parse().unwrap();
        let code: u16 = *code;
        let kind = StatusCodeKind::from(code);

        Self { kind, code }
    }

    pub fn is_failure(&self) -> bool {
        self.code > 399 && self.code < 599
    }
}
