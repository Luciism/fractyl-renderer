mod cache;

use std::{collections::HashMap, string::FromUtf8Error};

use image::{
    ImageBuffer, ImageError, ImageFormat, ImageReader, Rgba,
    imageops::{self, overlay},
};
use log::{error, info, warn};
use resvg::{
    tiny_skia::{Pixmap, PixmapMut},
    usvg::{self, Options, Transform},
};
use serde::Deserialize;

use crate::schema::{Fragment, Schema, SchemaFragmentType};

pub type PlaceholderMap = HashMap<String, String>;
pub type TextPlaceholderMap = HashMap<String, Vec<TextSpan>>;

#[derive(Deserialize, Debug, Clone)]
pub struct PlaceholderValues {
    pub text: TextPlaceholderMap,
    pub images: PlaceholderMap,
    pub shapes: PlaceholderMap,
}

impl PlaceholderValues {
    pub fn text(&self) -> PlaceholderMap {
        let mut map = HashMap::new();

        for (id, spans) in &self.text {
            let values: Vec<String> = spans.iter().map(|span| span.to_tspan()).collect();
            map.insert(id.clone(), values.join(""));
        }

        info!("{map:#?}");
        map
    }

    pub fn images(&self) -> PlaceholderMap {
        self.images.clone()
    }

    pub fn shapes(&self) -> PlaceholderMap {
        self.shapes.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TextSpan {
    pub value: String,
    pub fill: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u32>,
    pub font_family: Option<String>,
}

impl TextSpan {
    fn escaped_value(&self) -> String {
        self.value.replace("&", "&amp;").replace(">", "&gt;").replace("<", "&lt;")
    }

    pub fn to_tspan(&self) -> String {
        let mut attributes = vec![];

        if let Some(fill) = &self.fill {
            attributes.push(format!("fill=\"{fill}\""));
        }

        if let Some(font_size) = self.font_size {
            attributes.push(format!("font-size=\"{font_size}\""));
        }

        if let Some(font_weight) = self.font_weight {
            attributes.push(format!("font-weight=\"{font_weight}\""));
        }

        if let Some(font_family) = &self.font_family {
            attributes.push(format!("font-family=\"{font_family}\""));
        }

        format!("<tspan {} xml:space=\"preserve\">{}</tspan>",
            attributes.join(" "),
            self.escaped_value()
        )
    }
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

#[derive(Debug)]
pub struct Renderer<'a> {
    schema: Schema,
    used_placeholders: UsedPlaceholders,
    values: PlaceholderValues,
    usvg_options: &'a Options<'a>,
}

impl<'a> Renderer<'a> {
    pub fn build(
        schema: Schema,
        values: PlaceholderValues,
        options: &'a usvg::Options<'a>,
    ) -> Self {
        Renderer {
            schema,
            used_placeholders: UsedPlaceholders::new(),
            values,
            usvg_options: options,
        }
    }

    fn get_x(&self, x: u32) -> i64 {
        (self.schema.content_box.raster_x + x).into()
    }

    fn get_y(&self, y: u32) -> i64 {
        (self.schema.content_box.raster_y + y).into()
    }

    fn create_composite_pixmap(&self) -> Result<Pixmap, RenderingError> {
        let raster_size = &self.schema.raster_size;

        let pixmap = Pixmap::new(raster_size.width, raster_size.height)
            .ok_or(RenderingError::PixmapAllocationError)?;

        Ok(pixmap)
    }

    fn render_svg(
        &self,
        svg_code: &str,
        x: f32,
        y: f32,
        pixmap_mut: &mut PixmapMut,
    ) -> Result<(), RenderingError> {
        // TODO: use cache
        let tree = usvg::Tree::from_str(svg_code, &self.usvg_options)
            .map_err(|e| RenderingError::SVGParseError(e))?;

        resvg::render(&tree, Transform::from_translate(x, y), pixmap_mut);

        Ok(())
    }

    fn pixmap_to_png(pixmap: Pixmap) -> Result<ImgBuf, RenderingError> {
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
        fragments_pixmap_mut: &mut PixmapMut,
    ) -> Result<(), RenderingError> {
        for fragment in fragments {
            let (placeholder_values, used_placeholders) = match fragment.fragment_type() {
                SchemaFragmentType::Text => (&self.values.text(), &mut self.used_placeholders.text),
                SchemaFragmentType::Image => {
                    (&self.values.images(), &mut self.used_placeholders.images)
                }
                SchemaFragmentType::Shape => {
                    (&self.values.shapes(), &mut self.used_placeholders.shapes)
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

            let position = fragment.position();

            self.render_svg(
                &svg_code,
                self.get_x(position.x) as f32,
                self.get_y(position.y) as f32,
                fragments_pixmap_mut,
            )?;
        }

        Ok(())
    }

    fn render_to_background(&mut self, background_img: &mut ImgBuf) -> Result<(), RenderingError> {
        let mut fragments_pixmap = self.create_composite_pixmap()?;
        let mut fragments_pixmap_mut = fragments_pixmap.as_mut();
        self.render_fragments(
            &self.schema.fragments.text.clone(),
            &mut fragments_pixmap_mut,
        )?;
        self.render_fragments(
            &self.schema.fragments.images.clone(),
            &mut fragments_pixmap_mut,
        )?;
        self.render_fragments(
            &self.schema.fragments.shapes.clone(),
            &mut fragments_pixmap_mut,
        )?;

        let fragments_img = Renderer::pixmap_to_png(fragments_pixmap)?;

        overlay(background_img, &fragments_img, 0, 0);

        Ok(())
    }

    pub fn blend_rgba(src: [u8; 4], dst: [u8; 4]) -> [u8; 4] {
        let src_a = src[3] as f32 / 255.0;
        let dst_a = dst[3] as f32 / 255.0;

        let out_a = src_a + dst_a * (1.0 - src_a);

        if out_a == 0.0 {
            return [0, 0, 0, 0];
        }

        let blend = |s: u8, d: u8| -> u8 {
            let s = s as f32 / 255.0;
            let d = d as f32 / 255.0;

            (((s * src_a + d * dst_a * (1.0 - src_a)) / out_a) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8
        };

        [
            blend(src[0], dst[0]),
            blend(src[1], dst[1]),
            blend(src[2], dst[2]),
            (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    fn overlay_with_mask(bottom: &mut ImgBuf, top: &ImgBuf, mask: &ImgBuf) {
        for ((bottom_pixel, top_pixel), mask_pixel) in
            bottom.pixels_mut().zip(top.pixels()).zip(mask.pixels())
        {
            if mask_pixel[0] != 255 {
                *bottom_pixel = *top_pixel;
                continue;
            }

            let sa = top_pixel[3];
            if sa == 255 {
                *bottom_pixel = *top_pixel;
            } else if sa != 0 {
                *bottom_pixel = Rgba(Renderer::blend_rgba(top_pixel.0, bottom_pixel.0));
            }
        }
    }

    pub fn load_rgba_img_buf(&self, schema_asset_fp: &str) -> Result<ImgBuf, RenderingError> {
        Ok(image::open(
            &self
                .schema
                .absolute_asset_path(schema_asset_fp)
                .map_err(|e| RenderingError::FileSystemError(e))?,
        )
        .map_err(|e| RenderingError::ImageError(e))?
        .to_rgba8())
    }

    pub fn create_translucent_base(
        &mut self,
        mut background_img: ImgBuf,
    ) -> Result<ImgBuf, RenderingError> {
        let buf_cache = cache::IMG_BUF_CACHE.lock();

        let (translucent_base, mask) = match buf_cache {
            Ok(mut buf_cache) => (
                match buf_cache.filter_for(
                    Some(self.schema.id.clone()),
                    Some(self.schema.static_base.translucent.clone()),
                ) {
                    Some(translucent_base) => translucent_base,
                    None => {
                        let img = self.load_rgba_img_buf(&self.schema.static_base.translucent)?;
                        buf_cache.add_entry(
                            &self.schema.id,
                            &self.schema.static_base.translucent,
                            img.clone(),
                        );
                        img
                    }
                },
                match buf_cache.filter_for(
                    Some(self.schema.id.clone()),
                    Some(self.schema.static_base.mask.clone()),
                ) {
                    Some(mask) => mask,
                    None => {
                        let img = self.load_rgba_img_buf(&self.schema.static_base.mask)?;
                        buf_cache.add_entry(
                            &self.schema.id,
                            &self.schema.static_base.mask,
                            img.clone(),
                        );
                        img
                    }
                },
            ),
            Err(err) => {
                error!("Failed to acquire lock on cache: {err}");
                (
                    self.load_rgba_img_buf(&self.schema.static_base.translucent)?,
                    self.load_rgba_img_buf(&self.schema.static_base.mask)?,
                )
            }
        };

        let (width, height) = mask.dimensions();
        if background_img.width() > width || background_img.height() > height {
            background_img = imageops::crop(&mut background_img, 0, 0, width, height).to_image();
        }

        Renderer::overlay_with_mask(&mut background_img, &translucent_base, &mask);
        // Renderer::apply_binary_mask(&mut background_img, &mask);
        // overlay(&mut background_img, &translucent_base, 0, 0);

        Ok(background_img)
    }

    pub fn render_opaque(&mut self) -> Result<ImgBuf, RenderingError> {
        let buf_cache = cache::IMG_BUF_CACHE.lock();

        let opaque_base_src = &self.schema.static_base.opaque;
        let mut opaque_base = match buf_cache {
            Ok(mut buf_cache) => match buf_cache
                .filter_for(Some(self.schema.id.clone()), Some(opaque_base_src.clone()))
            {
                Some(opaque_base) => opaque_base,
                None => {
                    let img = self.load_rgba_img_buf(opaque_base_src)?;
                    buf_cache.add_entry(&self.schema.id, opaque_base_src, img.clone());
                    img
                }
            },
            Err(err) => {
                error!("Failed to acquire lock on cache: {err}");
                self.load_rgba_img_buf(opaque_base_src)?
            }
        };

        self.render_to_background(&mut opaque_base)?;
        Ok(opaque_base)
    }

    pub fn render_translucent(&mut self, background_img: ImgBuf) -> Result<ImgBuf, RenderingError> {
        let mut static_base = self.create_translucent_base(background_img)?;
        self.render_to_background(&mut static_base)?;
        Ok(static_base)
    }
}
