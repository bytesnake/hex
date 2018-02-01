use std::{fmt, result};
use failure::{Error, Context, Fail, Backtrace};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct MyError {
    inner: Context<ErrorKind>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Something failed inside the music library")]
    Music,
    #[fail(display = "Could not configure the application")]
    Configuration,
    #[fail(display = "Could not start the websocket server")]
    Server,
    #[fail(display = "Message parsing failed")]
    Parsing,
    #[fail(display = "Youtube downloader")]
    Youtube
}

impl Fail for MyError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

/*
impl MyError {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}*/

impl From<ErrorKind> for MyError {
    fn from(kind: ErrorKind) -> MyError {
        MyError { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for MyError {
    fn from(inner: Context<ErrorKind>) -> MyError {
        MyError { inner: inner }
    }
}
