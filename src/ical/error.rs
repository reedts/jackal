use std::error;
use std::fmt;
use std::convert::From;

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: Option<String>
}

#[derive(Debug)]
pub enum ErrorKind {
    CalendarParse,
    CalendarMissingKey,
    EventParse,
    EventMissingKey,
    TimeParse
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error { kind, message: None }
    }
    
    pub fn with_msg(mut self, message: &str) -> Self {
        self.message = Some(message.to_owned());
        self
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::new(kind)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(msg) => write!(f, "{}: {}", self.kind.as_str(), msg),
            None      => write!(f, "{}", self.kind.as_str())
        }
    }
}

impl error::Error for Error {}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::CalendarParse      => "invalid calendar format",
            ErrorKind::CalendarMissingKey => "missing key in calendar definition",
            ErrorKind::EventParse         => "invalid event format",
            ErrorKind::EventMissingKey    => "missing key in event definition",
            ErrorKind::TimeParse          => "invalid time format"
        }
    }
}
