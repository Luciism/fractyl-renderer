use std::{fs::canonicalize, io::BufReader, path::{Path, absolute}};
use std::path::{PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::SchemaError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaV1ContentBox {
    pub width: u32,
    pub height: u32,
    pub raster_x: u32,
    pub raster_y: u32,
}

#[derive(Deserialize, Debug)]
pub struct SchemaV1RasterSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Debug)]
pub struct SchemaV1StaticBase {
    pub opaque: String,
    pub translucent: String,
    pub mask: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SchemaV1Position {
    pub x: u32,
    pub y: u32,
}

impl SchemaV1Position {
    pub fn as_tuple(&self) -> (u32, u32) {
        (self.x, self.y)
    }
}

pub trait SchemaV1Fragment {
    fn src(&self) -> &String;
    fn position(&self) -> &SchemaV1Position;
    fn placeholders(&self) -> &Vec<String>;
    fn fragment_type(&self) -> SchemaV1FragmentType;
}

#[derive(Deserialize, Debug, Clone)]
pub struct SchemaV1ImageFragment {
    pub src: String,
    pub position: SchemaV1Position,
    pub placeholders: Vec<String>,
}

impl SchemaV1Fragment for SchemaV1ImageFragment {
    fn src(&self) -> &String { &self.src }
    fn position(&self) -> &SchemaV1Position { &self.position }
    fn placeholders(&self) -> &Vec<String> { &self.placeholders }
    fn fragment_type(&self) -> SchemaV1FragmentType { SchemaV1FragmentType::Image }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SchemaV1TextFragment {
    pub src: String,
    pub position: SchemaV1Position,
    pub placeholders: Vec<String>,
}

impl SchemaV1Fragment for SchemaV1TextFragment {
    fn src(&self) -> &String { &self.src }
    fn position(&self) -> &SchemaV1Position { &self.position }
    fn placeholders(&self) -> &Vec<String> { &self.placeholders }
    fn fragment_type(&self) -> SchemaV1FragmentType { SchemaV1FragmentType::Text }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SchemaV1Mode {
    Fixed,
    Dynamic,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SchemaV1FragmentType {
    Text,
    Image,
    Shape,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SchemaV1ShapeFragment {
    pub src: String,
    pub position: SchemaV1Position,
    pub placeholders: Vec<String>,
    pub width_mode: SchemaV1Mode,
    pub height_mode: SchemaV1Mode,
    pub color_mode: SchemaV1Mode,
}

impl SchemaV1Fragment for SchemaV1ShapeFragment {
    fn src(&self) -> &String { &self.src }
    fn position(&self) -> &SchemaV1Position { &self.position }
    fn placeholders(&self) -> &Vec<String> { &self.placeholders }
    fn fragment_type(&self) -> SchemaV1FragmentType { SchemaV1FragmentType::Shape }
}


#[derive(Deserialize, Debug, Clone)]
pub struct SchemaV1Fragments {
    pub images: Vec<SchemaV1ImageFragment>,
    pub text: Vec<SchemaV1TextFragment>,
    pub shapes: Vec<SchemaV1ShapeFragment>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaV1 {
    pub schema_file: String,

    pub schema_version: u64,
    pub name: String,
    pub content_box: SchemaV1ContentBox,
    pub raster_size: SchemaV1RasterSize,
    pub static_base: SchemaV1StaticBase,
    pub fragments: SchemaV1Fragments,
}

impl SchemaV1 {
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

    pub fn read_schema_asset_file(&self, specified_fp: &str) -> Result<Vec<u8>, std::io::Error> {
        let path = self.absolute_asset_path(specified_fp)?;
        std::fs::read(path)
    }
}



pub fn load_schema_v1(schema_fp: &str, mut schema_json: Value) -> Result<SchemaV1, SchemaError> {
    schema_json["schemaFile"] = Value::String(schema_fp.to_string());
    let schema: SchemaV1 =
        serde_json::from_value(schema_json).map_err(|e| SchemaError::SchemaDecodeError(e))?;

    Ok(schema)
}

