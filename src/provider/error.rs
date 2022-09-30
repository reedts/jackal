use nom;
use std::convert::From;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum ErrorKind {
    CalendarParse,
    CalendarMissingKey,
    EventParse,
    EventMissingKey,
    TimeParse,
    DateParse,
    DurationParse,
    RecurRuleParse,
    ParseError,
    IOError(io::Error),
}

impl Error {
    pub fn new(kind: ErrorKind, msg: &str) -> Self {
        Error {
            kind,
            message: Some(msg.to_owned()),
        }
    }

    pub fn with_msg(mut self, message: &str) -> Self {
        self.message = Some(message.to_owned());
        self
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,
            message: None,
        }
    }
}

impl From<io::ErrorKind> for Error {
    fn from(kind: io::ErrorKind) -> Error {
        Error::from(io::Error::from(kind))
    }
}

impl From<chrono::ParseError> for Error {
    fn from(parse_error: chrono::ParseError) -> Error {
        Error::new(
            ErrorKind::TimeParse,
            format!("Could not parse timestamp: {}", parse_error).as_str(),
        )
    }
}

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Error {
        Error::from(ErrorKind::IOError(io_error))
    }
}

impl<E: std::fmt::Debug> From<nom::Err<E>> for Error {
    fn from(error: nom::Err<E>) -> Self {
        Error::new(
            ErrorKind::ParseError,
            &format!("Error while parsing: {}", error),
        )
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        if let ErrorKind::IOError(err) = err.kind {
            err
        } else {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                err.message.unwrap_or("invalid format".to_owned()),
            )
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(msg) => write!(f, "{}: {}", self.kind.as_str(), msg),
            None => write!(f, "{}", self.kind.as_str()),
        }
    }
}

impl error::Error for Error {}

impl ErrorKind {
    pub fn as_str(&self) -> String {
        match self {
            ErrorKind::CalendarParse => "invalid calendar format".to_owned(),
            ErrorKind::CalendarMissingKey => "missing key in calendar definition".to_owned(),
            ErrorKind::EventParse => "invalid event format".to_owned(),
            ErrorKind::EventMissingKey => "missing key in event definition".to_owned(),
            ErrorKind::TimeParse => "invalid time format".to_owned(),
            ErrorKind::DateParse => "invalid date format".to_owned(),
            ErrorKind::DurationParse => "invalid duration format".to_owned(),
            ErrorKind::RecurRuleParse => "invalid reccurrence format".to_owned(),
            ErrorKind::ParseError => "invalid format".to_owned(),
            ErrorKind::IOError(err) => err.to_string(),
        }
    }
}
