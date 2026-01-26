use std::{collections::HashMap, string::FromUtf8Error};

use image::{ImageBuffer, ImageError, ImageFormat, ImageReader, Rgba, imageops::{self, overlay}};
use log::warn;
use resvg::{
    tiny_skia::Pixmap,
    usvg::{self, Transform},
};
use serde::Deserialize;

use crate::schema::{Fragment, Schema, SchemaFragmentType};

pub type PlaceholderMap = HashMap<String, String>;

pub struct XY(u32, u32);
impl XY {
    pub fn from_tuple(tuple: (u32, u32)) -> Self {
        XY(tuple.0, tuple.1)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlaceholderValues {
    pub text: PlaceholderMap,
    pub images: PlaceholderMap,
    pub shapes: PlaceholderMap,
}

#[derive(Debug)]
struct UsedPlaceholders {
    text: Vec<String>,
    images: Vec<String>,
    shapes: Vec<String>,
}

impl UsedPlaceholders {
    fn new() -> Self {
        UsedPlaceholders {
            text: vec![],
            images: vec![],
            shapes: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Renderer {
    schema: Schema,
    used_placeholders: UsedPlaceholders,
    values: PlaceholderValues,
}

#[derive(Debug)]
pub enum RenderingError {
    FileSystemError(std::io::Error),
    UTF8EncodingError(FromUtf8Error),
    SVGParseError(usvg::Error),
    PixmapAllocationError,
    ReadStaticPNGError,
    PngEncodeError,
    PngDecodeError(ImageError),
    ImageError(ImageError),
}

pub type ImgBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

impl Renderer {
    pub fn build(schema: Schema, values: PlaceholderValues) -> Self {
        Renderer {
            schema,
            used_placeholders: UsedPlaceholders::new(),
            values,
        }
    }

    fn get_x(&self, x: u32) -> i64 {
        (self.schema.content_box.raster_x + x).into()
    }

    fn get_y(&self, y: u32) -> i64 {
        (self.schema.content_box.raster_y + y).into()
    }

    fn svg_to_png(svg_code: &str) -> Result<ImgBuf, RenderingError> {
        let mut options = usvg::Options::default();
        options.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(svg_code, &options)
            .map_err(|e| RenderingError::SVGParseError(e))?;
        let size = tree.size().to_int_size();
        let mut pixmap = Pixmap::new(size.width(), size.height())
            .ok_or(RenderingError::PixmapAllocationError)?;

        let mut pixmap_mut = pixmap.as_mut();

        resvg::render(&tree, Transform::default(), &mut pixmap_mut);

        // Ok(pixmap_mut.to_owned())

        let encoded_png = pixmap
            .to_owned()
            .encode_png()
            .map_err(|_| RenderingError::PngEncodeError)?;
        let cursor = std::io::Cursor::new(encoded_png);

        let mut reader = ImageReader::new(cursor);
        reader.set_format(ImageFormat::Png);

        Ok(reader
            .decode()
            .map_err(|e| RenderingError::PngDecodeError(e))?
            .to_rgba8())
    }

    fn replace_placeholders(
        schema_placeholders: &Vec<String>,
        placeholder_values: &PlaceholderMap,
        mut svg_code: String,
        used_placeholders: &mut Vec<String>,
        mut unused_placeholders: Vec<String>,
    ) -> String {
        for (name, value) in placeholder_values {
            svg_code = svg_code.replace(&("{".to_string() + &name + "}"), &value);
            used_placeholders.push(name.to_string());

            if !schema_placeholders.contains(&name.to_string()) {
                warn!("Placeholder '{name}' is not specified in the schema!");
            }

            match unused_placeholders.iter().position(|p| p == name) {
                Some(index) => {
                    unused_placeholders.remove(index);
                }
                None => (),
            };
        }

        if unused_placeholders.len() > 0 {
            warn!("Unused placeholders: {}", unused_placeholders.join(", "));
        }

        svg_code
    }

    fn render_fragments<T: Fragment>(
        &mut self,
        fragments: &Vec<T>,
    ) -> Result<Vec<(XY, ImgBuf)>, RenderingError> {
        let mut imgs = vec![];

        for fragment in fragments {
            let (placeholder_values, used_placeholders) = match fragment.fragment_type() {
                SchemaFragmentType::Text => (&self.values.text, &mut self.used_placeholders.text),
                SchemaFragmentType::Image => {
                    (&self.values.images, &mut self.used_placeholders.images)
                }
                SchemaFragmentType::Shape => {
                    (&self.values.shapes, &mut self.used_placeholders.shapes)
                }
            };

            let svg_code = self
                .schema
                .read_schema_asset_file(&fragment.src())
                .map_err(|e| RenderingError::FileSystemError(e))?;
            let mut svg_code =
                String::from_utf8(svg_code).map_err(|e| RenderingError::UTF8EncodingError(e))?;

            let unused_placeholders = fragment.placeholders().clone();

            svg_code = Renderer::replace_placeholders(
                &fragment.placeholders(),
                &placeholder_values,
                svg_code,
                used_placeholders,
                unused_placeholders,
            );

            let img = Renderer::svg_to_png(&svg_code)?;
            imgs.push((XY::from_tuple(fragment.position().as_tuple()), img))
        }

        Ok(imgs)
    }

    fn render_to_background(&mut self, background_img: &mut ImgBuf) -> Result<(), RenderingError> {
        let mut img_bufs = self.render_fragments(&self.schema.fragments.text.clone())?;
        img_bufs.extend(self.render_fragments(&self.schema.fragments.shapes.clone())?);
        img_bufs.extend(self.render_fragments(&self.schema.fragments.images.clone())?);

        for (xy, img) in img_bufs {
            overlay(background_img, &img, self.get_x(xy.0), self.get_y(xy.1));
        }

        Ok(())
    }



    fn apply_binary_mask(base: &mut ImgBuf, mask: &ImgBuf) {
        for (out_pixel, mask_pixel) in base.pixels_mut().zip(mask.pixels()) {
            if mask_pixel[0] == 0 { // Black in mask
                *out_pixel = Rgba([0, 0, 0, 0]);
            }
        }
    }

    pub fn create_translucent_base(&mut self, mut background_img: ImgBuf) -> Result<ImgBuf, RenderingError> {
        let translucent_base = image::open(
            &self
                .schema
                .absolute_asset_path(&self.schema.static_base.translucent)
                .map_err(|e| RenderingError::FileSystemError(e))?,
        )
        .map_err(|e| RenderingError::ImageError(e))?
        .to_rgba8();

        let mask = image::open(
            &self
                .schema
                .absolute_asset_path(&self.schema.static_base.mask)
                .map_err(|e| RenderingError::FileSystemError(e))?,
        )
        .map_err(|e| RenderingError::ImageError(e))?
        .to_rgba8();

        let (width, height) = mask.dimensions();
        background_img = imageops::crop(&mut background_img, 0, 0, width, height).to_image();

        Renderer::apply_binary_mask(&mut background_img, &mask);

        overlay(&mut background_img, &translucent_base, 0, 0);
        Ok(background_img)
    }

    pub fn render_opaque(&mut self) -> Result<ImgBuf, RenderingError> {
        let mut static_base = image::open(
            &self
                .schema
                .absolute_asset_path(&self.schema.static_base.opaque)
                .map_err(|e| RenderingError::FileSystemError(e))?,
        )
        .map_err(|e| RenderingError::ImageError(e))?
        .to_rgba8();

        self.render_to_background(&mut static_base)?;
        Ok(static_base)
    }

    pub fn render_translucent(&mut self, background_img: ImgBuf) -> Result<ImgBuf, RenderingError> {
        let mut static_base = self.create_translucent_base(background_img)?;
        self.render_to_background(&mut static_base)?;
        Ok(static_base)
    }
}
