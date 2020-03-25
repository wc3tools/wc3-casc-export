//! Text file reader

pub mod bom;
mod parse;
mod types;

pub use self::parse::parse_sections;
pub use self::types::*;
