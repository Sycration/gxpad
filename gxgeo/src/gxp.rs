//! The .gxp document format: a JSON encoding of a [`Doc`].

use crate::model::Doc;

pub use serde_json::Error;

/// File extension of the format, without the dot.
pub const EXTENSION: &str = "gxp";

/// Encodes a document as compact .gxp text.
pub fn to_string(doc: &Doc) -> Result<String, Error> {
    serde_json::to_string(doc)
}

/// Encodes a document as indented .gxp text.
pub fn to_string_pretty(doc: &Doc) -> Result<String, Error> {
    serde_json::to_string_pretty(doc)
}

/// Decodes a document from .gxp text.
pub fn from_str(text: &str) -> Result<Doc, Error> {
    serde_json::from_str(text)
}
