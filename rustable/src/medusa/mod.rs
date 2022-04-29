//! Everything related to Medusa communication protocol.

pub mod attribute;
pub use attribute::{AttributeBytes, MedusaAttribute, MedusaAttributeHeader, MedusaAttributes};

pub mod config;
pub use config::{Config, ConfigBuilder};

mod constants;
pub use constants::{AccessType, HandlerFlags};

pub mod class;
pub use class::{MedusaClass, MedusaClassHeader};

pub mod context;
pub use context::Context;

pub mod event;
pub use event::{MedusaEvtype, MedusaEvtypeHeader, Monitoring};

pub mod error;
pub use error::{AttributeError, CommunicationError, ConfigError, ConnectionError, ReaderError};

pub mod handler;
pub use handler::{
    CustomHandler, EventHandler, EventHandlerBuilder, Handler, HandlerArgs, HandlerData,
};

pub mod mcp;
pub use mcp::Connection;

mod parser;

mod reader;
use reader::{AsyncReader, NativeByteOrderReader};

pub mod request;
pub use request::{
    AuthRequestData, DecisionAnswer, FetchAnswer, MedusaAnswer, MedusaRequest, RequestType,
    UpdateAnswer,
};

mod space;
pub use space::{Space, SpaceBuilder, VirtualSpace};

pub mod tree;
pub use tree::{Node, NodeBuilder, Tree, TreeBuilder};

mod writer;
use writer::Writer;

type Command = u32;
