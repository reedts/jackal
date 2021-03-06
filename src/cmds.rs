use std::error;
use std::fmt;
use std::result;

#[derive(Debug, Clone, Copy)]
pub enum Cmd {
    Noop,
    NextDay,
    PrevDay,
    NextWeek,
    PrevWeek,
    Exit,
}

pub type Result = result::Result<Cmd, CmdFailed>;

#[derive(Debug, Clone)]
pub struct CmdFailed;

impl fmt::Display for CmdFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error executing command")
    }
}

impl error::Error for CmdFailed {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
