/// Schema v1 model

use std::path::PathBuf;
use std::path::absolute;

use serde::Deserialize;
use serde_json::Value;

use super::SchemaError;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/** The bounding box of the content to be rendered within a raster. */
pub struct ContentBox {
    /** The total width of the content box. */
    pub width: u32,
    /** The total height of the content box. */
    pub height: u32,
    /** The X position of the content box within the raster. */
    pub raster_x: u32,
    /** The Y position of the content box within the raster. */
    pub raster_y: u32,
}

#[derive(Deserialize, Debug, Clone)]
/** The size of the master raster image. */
pub struct RasterSize {
    /** The total width of the raster. */
    pub width: u32,
    /** The total height of the raster. */
    pub height: u32,
}

#[derive(Deserialize, Debug, Clone)]
/** The paths for the static assets. */
pub struct StaticBase {
    /** The path to the opaque base image. */
    pub opaque: String,
    /** The path to the translucent base image. */
    pub translucent: String,
    /** The path to the mask image for clipping background images. */
    pub mask: String,
}

#[derive(Deserialize, Debug, Clone)]
/** The XY position of an element. */
pub struct Position {
    /** The X position of an element. */
    pub x: u32,
    /** The Y position of an element. */
    pub y: u32,
}

impl Position {
    /** Convert to an (x, y) tuple. */
    pub fn as_tuple(&self) -> (u32, u32) {
        (self.x, self.y)
    }
}

/** Common methods for fragments */
pub trait Fragment {
    /** The path for the fragment SVG file. */
    fn src(&self) -> &String;
    /** The position within the content box to render the fragment. */
    fn position(&self) -> &Position;
    /** The placeholders within the fragment that are expected to be replaced. */
    fn placeholders(&self) -> &Vec<String>;
    /** The type of fragment. */
    fn fragment_type(&self) -> FragmentType;
}

#[derive(Deserialize, Debug, Clone)]
/** The associated data for an image fragment. */
pub struct ImageFragment {
    /** The path for the fragment SVG file. */
    pub src: String,
    /** The position within the content box to render the fragment. */
    pub position: Position,
    /** The placeholders within the fragment that are expected to be replaced. */
    pub placeholders: Vec<String>,
}

impl Fragment for ImageFragment {
    fn src(&self) -> &String {
        &self.src
    }
    fn position(&self) -> &Position {
        &self.position
    }
    fn placeholders(&self) -> &Vec<String> {
        &self.placeholders
    }
    fn fragment_type(&self) -> FragmentType {
        FragmentType::Image
    }
}

#[derive(Deserialize, Debug, Clone)]
/** The associated data for a text fragment. */
pub struct TextFragment {
    /** The path for the fragment SVG file. */
    pub src: String,
    /** The position within the content box to render the fragment. */
    pub position: Position,
    /** The placeholders within the fragment that are expected to be replaced. */
    pub placeholders: Vec<String>,
}

impl Fragment for TextFragment {
    fn src(&self) -> &String {
        &self.src
    }
    fn position(&self) -> &Position {
        &self.position
    }
    fn placeholders(&self) -> &Vec<String> {
        &self.placeholders
    }
    fn fragment_type(&self) -> FragmentType {
        FragmentType::Text
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
/** Whether a attribute is fixed or dynamic */
pub enum Mode {
    /** Fixed means that a value is predefined at export time. */
    Fixed,
    /** Dynamic means that a value must be specified at render time. */
    Dynamic,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
/** The type of fragment */
pub enum FragmentType {
    Text,
    Image,
    Shape,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/** The associated data for a shape fragment. */
pub struct ShapeFragment {
    /** The path for the fragment SVG file. */
    pub src: String,
    /** The position within the content box to render the fragment. */
    pub position: Position,
    /** The placeholders within the fragment that are expected to be replaced. */
    pub placeholders: Vec<String>,
    /** Whether the width of the shape is fixed or dynamic. */
    pub width_mode: Mode,
    /** Whether the height of the shape is fixed or dynamic. */
    pub height_mode: Mode,
    /** Whether the color of the shape is fixed or dynamic. */
    pub color_mode: Mode,
}

impl Fragment for ShapeFragment {
    fn src(&self) -> &String {
        &self.src
    }
    fn position(&self) -> &Position {
        &self.position
    }
    fn placeholders(&self) -> &Vec<String> {
        &self.placeholders
    }
    fn fragment_type(&self) -> FragmentType {
        FragmentType::Shape
    }
}

#[derive(Deserialize, Debug, Clone)]
/** All of the fragments for the render. */
pub struct DynamicFragments {
    /** All of the image fragments for the render. */
    pub images: Vec<ImageFragment>,
    /** All of the text fragments for the render. */
    pub text: Vec<TextFragment>,
    /** All of the shape fragments for the render. */
    pub shapes: Vec<ShapeFragment>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/** v1: The schema that determines the layout of the render and all of its elements. */
pub struct Schema {
    /** The path to the schema file. */
    pub schema_file: String,

    /** The version of the schema. */
    pub schema_version: u64,
    /** The ID of the schema. */
    pub id: String,
    /** The name of the schema. */
    pub name: String,
    /** The content box of the schema. This is what fragments are positioned relative to. */
    pub content_box: ContentBox,
    /** The actual size of the raster. */
    pub raster_size: RasterSize,
    /** The paths to the static base images. */
    pub static_base: StaticBase,
    /** All of the fragments for the render. */
    pub fragments: DynamicFragments,
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
Load json data into the schema v1 model.

# Parameters

- `schema_fp`: The filepath of the schema file.
- `schema_json`: The serde_json object loaded from the schema file.
*/
pub fn load_schema_v1(schema_fp: &str, mut schema_json: Value) -> Result<Schema, SchemaError> {
    schema_json["schemaFile"] = Value::String(schema_fp.to_string());
    let schema: Schema =
        serde_json::from_value(schema_json).map_err(|e| SchemaError::SchemaDecodeError(e))?;

    Ok(schema)
}
