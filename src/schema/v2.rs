/// Schema v2 model

use std::path::PathBuf;
use std::path::absolute;

use serde::Deserialize;
use serde_json::Value;

use super::SchemaError;
pub use super::v1::{ContentBox, RasterSize, StaticBase, Fragment, FragmentType, Position, DynamicFragments, ImageFragment, TextFragment, ShapeFragment, Mode};


#[derive(Deserialize, Debug, Clone)]
pub struct LayoutContent {
    /** The content box of the schema. This is what fragments are positioned relative to. */
    pub content_box: ContentBox,
    /** The actual size of the raster. */
    pub raster_size: RasterSize,
    /** The paths to the static base images. */
    pub static_base: StaticBase,
    /** All of the fragments for the render. */
    pub fragments: DynamicFragments,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Layout {
    pub id: u32,
    pub name: String,
    #[serde(rename = "camelCase")]
    pub is_default: bool,
    pub scale: f32,
    pub layout: LayoutContent
}


#[derive(Deserialize, Debug, Clone)]
/** The associated data for a variable. */
pub struct Variable {
    /** The name of the variable (includes the collection name) **/
    pub name: String,
    /** The value of the variable. */
    pub value: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/** v2: The schema that determines the layout of the render and all of its elements. */
pub struct Schema {
    /** The path to the schema file. */
    pub schema_file: String,
    /** The version of the schema. */
    pub schema_version: u64,
    /** The ID of the schema. */
    pub id: String,
    /** The name of the schema. */
    pub name: String,
    /** All of the layouts for the render. */
    pub layouts: Vec<Layout>,
    /** All of the variables for the render. */
    pub variables: Vec<Variable>
}

impl Schema {
    /** Get the absolute path for an asset specified by the schema. */
    pub fn absolute_asset_path(&self, specified_fp: &str) -> Result<PathBuf, std::io::Error> {
        let path = absolute(&format!("{}/../{}", self.schema_file, specified_fp))?;

        let mut result = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    result.pop();
                }
                std::path::Component::CurDir => {}
                other => result.push(other),
            }
        }

        Ok(result)
    }

    /** Read the contents of an asset file specified by the schema. */
    pub fn read_schema_asset_file(&self, specified_fp: &str) -> Result<Vec<u8>, std::io::Error> {
        let path = self.absolute_asset_path(specified_fp)?;
        std::fs::read(path)
    }
}

/**
Load json data into the schema v2 model.

# Parameters

- `schema_fp`: The filepath of the schema file.
- `schema_json`: The serde_json object loaded from the schema file.
*/
pub fn load_schema_v2(schema_fp: &str, mut schema_json: Value) -> Result<Schema, SchemaError> {
    schema_json["schemaFile"] = Value::String(schema_fp.to_string());
    let schema: Schema =
        serde_json::from_value(schema_json).map_err(|e| SchemaError::SchemaDecodeError(e))?;

    Ok(schema)
}
