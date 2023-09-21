//! Schema for a Pywr model.
//!
//! Schema definition for a Pywr model.
//!
//! Serializing and deserializing is accomplished using [`serde`].
//!
pub mod data_tables;
pub mod edge;
mod error;
pub mod metric_sets;
pub mod model;
pub mod nodes;
pub mod outputs;
pub mod parameters;

pub use error::ConversionError;
pub use model::PywrModel;
