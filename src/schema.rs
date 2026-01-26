// For future schema versions, a migration system shall be implemented.

#[allow(unused)]
mod v1;

use serde_json::Value;
use v1::SchemaV1;

pub use v1::SchemaV1Fragment as Fragment;
use v1::{
    SchemaV1FragmentType, SchemaV1ImageFragment, SchemaV1ShapeFragment, SchemaV1TextFragment,
};

#[derive(Debug)]
pub enum SchemaError {
    FileNotFoundError,
    FileReadError(std::io::Error),
    SchemaDecodeError(serde_json::Error),
    MalformedSchema(String),
    UnknownVersion(u64),
}

pub type Schema = SchemaV1;

pub type SchemaTextFragment = SchemaV1TextFragment;
pub type SchemaImageFragment = SchemaV1ImageFragment;
pub type SchemaShapeFragment = SchemaV1ShapeFragment;
pub type SchemaFragmentType = SchemaV1FragmentType;

pub fn load_schema_from_file(schema_fp: &str) -> Result<Schema, SchemaError> {
    if !std::fs::exists(schema_fp).map_err(|e| SchemaError::FileReadError(e))? {
        return Err(SchemaError::FileNotFoundError);
    }

    let content = std::fs::read_to_string(schema_fp).map_err(|e| SchemaError::FileReadError(e))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| SchemaError::SchemaDecodeError(e))?;

    let schema_version = json["schemaVersion"]
        .as_u64()
        .ok_or(SchemaError::MalformedSchema(
            "Missing 'schemaVersion".to_string(),
        ))?;

    match schema_version {
        1 => Ok(v1::load_schema_v1(schema_fp, json)?),
        _ => Err(SchemaError::UnknownVersion(schema_version)),
    }
}
