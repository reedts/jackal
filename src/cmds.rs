use std::convert::From;
use std::error;
use std::fmt;
use std::io;
use std::result;

#[derive(Debug, Clone, Copy)]
pub enum Cmd {
    Noop,
    NextDay,
    PrevDay,
    NextWeek,
    PrevWeek,
    NextEvent,
    PrevEvent,
    Exit,
}

pub type CmdResult = result::Result<Cmd, CmdError>;

#[derive(Debug, Clone)]
pub struct CmdError {
    message: Option<String>,
    kind: io::ErrorKind,
}

impl Default for CmdError {
    fn default() -> Self {
        CmdError {
            message: None,
            kind: io::ErrorKind::Other,
        }
    }
}

impl CmdError {
    pub fn new(message: String) -> Self {
        CmdError {
            message: Some(message),
            kind: io::ErrorKind::Other,
        }
    }

    pub fn with_msg(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {:#?}",
            self.message
                .as_ref()
                .unwrap_or(&"Error executing command".to_owned()),
            self.kind
        )
    }
}

impl error::Error for CmdError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<CmdError> for io::Error {
    fn from(error: CmdError) -> Self {
        io::Error::from(error.kind)
    }
}
