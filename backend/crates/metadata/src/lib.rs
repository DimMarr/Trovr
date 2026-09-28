//! Nodes/versions/permissions domain logic and sqlx queries.

mod error;
mod node;
mod validation;

pub use error::MetadataError;
pub use node::{FileVersion, NewContent, NewFile, Node, NodeType};
