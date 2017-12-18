use std::{fmt, result};

pub type Result<T> = result::Result<T, Error>;

pub enum Error {
    InvalidFile,
    Internal,
    RequestAborted,
    Parsing,
    OpusEncode,
    CommandFailed,
    AcousticID(String),
    AlreadyExists
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidFile => write!(f, "The file was not found!"),
            Error::Internal => write!(f, "An internal error occurred!"),
            Error::RequestAborted => write!(f, "The request was aborted!"),
            Error::Parsing => write!(f, "Parsing failed!"),
            Error::OpusEncode => write!(f, "The encoding process of OPUS has failed!"),
            Error::CommandFailed => write!(f, "Execution of a command failed!"),
            Error::AcousticID(ref err) => write!(f, "Requesting metadata from the acousticid webservice failed: {}", err),
            Error::AlreadyExists => write!(f, "The audio file does already exist!")
        }
    }
}
