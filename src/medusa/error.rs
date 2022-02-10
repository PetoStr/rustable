use crate::medusa::Command;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TreeError {
    #[error(transparent)]
    InvalidRegex(#[from] regex::Error),
    #[error("unexpected call to end_node() before begin_node()")]
    UnexpectedEndNode,
    #[error("end_node() was not called after begin_node()")]
    NotAtRootLevel,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ReaderError {
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error("{0}")]
    ParseError(String),
    #[error("unknown class with id 0x{0:x}")]
    UnknownClass(u64),
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
    UnsupportedVersion(u64),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CommunicationError {
    #[error(transparent)]
    IOError(#[from] tokio::io::Error),
    #[error(transparent)]
    ReaderError(#[from] ReaderError),
    #[error("unknown command: 0x{0:x}")]
    UnknownCommand(Command),
    #[error("unknown access type: 0x{0:x}")]
    UnknownAccessType(u64),
    #[error("unknown subject type: 0x{0:x}")]
    UnknownSubjectType(u64),
    #[error("unknown object type: 0x{0:x}")]
    UnknownObjectType(u64),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AttributeError {
    #[error("unknown attribute: \"{0}\"")]
    UnknownAttribute(String),
    #[error("cannot modify read-only attribute: \"{0}\"")]
    ModifyReadOnlyError(String),
}
