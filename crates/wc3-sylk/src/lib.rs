//! Sylk (SLK) file reader

mod parse;
mod reader;
mod types;

pub use self::reader::{Cell, Row, SylkReader, SylkRows};
pub use self::types::*;
