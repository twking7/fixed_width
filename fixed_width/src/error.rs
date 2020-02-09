use crate::{de::DeserializeError, ser::SerializeError};
use std::{error::Error as StdError, fmt, io, string};

/// An error produced while parsing fixed width data.
#[derive(Debug)]
pub enum Error {
    /// An IO error occured while reading the data.
    IOError(io::Error),
    /// A record could not be converted into valid UTF-8.
    FormatError(string::FromUtf8Error),
    /// An error occurred during deserialization.
    DeserializeError(DeserializeError),
    /// An error occurred during serialization.
    SerializeError(SerializeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IOError(ref e) => write!(f, "{}", e),
            Error::FormatError(ref e) => write!(f, "{}", e),
            Error::DeserializeError(ref e) => write!(f, "{}", e),
            Error::SerializeError(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<DeserializeError> for Error {
    fn from(e: DeserializeError) -> Self {
        Error::DeserializeError(e)
    }
}

impl From<SerializeError> for Error {
    fn from(e: SerializeError) -> Self {
        Error::SerializeError(e)
    }
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match self {
            Error::IOError(ref e) => Some(e),
            Error::FormatError(ref e) => Some(e),
            Error::DeserializeError(ref e) => Some(e),
            Error::SerializeError(ref e) => Some(e),
        }
    }
}
