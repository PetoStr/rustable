use crate::medusa::Command;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ConfigError {
    #[error(transparent)]
    InvalidRegexError(#[from] regex::Error),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ReaderError {
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error("{0}")]
    ParseError(String),
    #[error("unknown class with id 0x{0:x}")]
    UnknownClassError(u64),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ConnectionError {
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReaderError(#[from] ReaderError),
    #[error("unknown byte order for greeting: 0x{0:x}")]
    UnknownByteOrder(u64),
    #[error("protocol version {0} is not supported")]
    UnsupportedVersionError(u64),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CommunicationError {
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReaderError(#[from] ReaderError),
    #[error("unknown command: 0x{0:x}")]
    UnknownCommandError(Command),
    #[error("unknown access type: 0x{0:x}")]
    UnknownAccessTypeError(u64),
    #[error("unknown subject type: 0x{0:x}")]
    UnknownSubjectTypeError(u64),
    #[error("unknown object type: 0x{0:x}")]
    UnknownObjectTypeError(u64),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AttributeError {
    #[error("unknown attribute: \"{0}\"")]
    UnknownAttributeError(String),
    #[error("cannot modify read-only attribute: \"{0}\"")]
    ModifyReadOnlyError(String),
}
