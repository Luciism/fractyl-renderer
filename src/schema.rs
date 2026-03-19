// For future schema versions, a migration system shall be implemented.

mod v1;
mod v2;
pub use v1::Fragment;

use serde_json::Value;

#[derive(Debug)]
/** Schema load errors. */
pub enum SchemaError {
    /// The schema file was not found.
    FileNotFoundError,
    /// The schema file could not be read.
    FileReadError(std::io::Error),
    /// The schema file could not be decoded.
    SchemaDecodeError(serde_json::Error),
    /// The schema file is malformed.
    MalformedSchema(String),
    /// Unknown schema version.
    UnknownVersion(u64),
    MissingDefaultLayout,
}

pub type Schema = v2::Schema;
pub type SchemaLayout = v2::Layout;
pub type SchemaTextFragment = v2::TextFragment;
pub type SchemaImageFragment = v2::ImageFragment;
pub type SchemaShapeFragment = v2::ShapeFragment;
pub type SchemaFragmentType = v2::FragmentType;

/// Load a schema from a file.
///
/// # Parameters
/// - `schema_fp` - The path to the schema file.
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
        1 => Ok(v1::load_schema_v1(schema_fp, json)?.migrate()),
        2 => Ok(v2::load_schema_v2(schema_fp, json)?),
        _ => Err(SchemaError::UnknownVersion(schema_version)),
    }
}
