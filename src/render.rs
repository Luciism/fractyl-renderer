use std::string::FromUtf8Error;

use image::{
    ImageBuffer, ImageError, ImageFormat, ImageReader, Rgba,
    imageops::{FilterType, crop, overlay, resize},
};
use log::warn;
use regex::Regex;
use resvg::{
    tiny_skia::{Pixmap, PixmapMut},
    usvg::{self, Options, Transform},
};

use crate::schema::{Fragment, Schema, SchemaFragmentType, SchemaLayout};

use crate::placeholders::{PlaceholderValueMap, PlaceholderValues, UsedPlaceholders};

#[derive(Debug)]
/** Rendering errors. */
pub enum RenderingError {
    FileSystemError(std::io::Error),
    UTF8EncodingError(FromUtf8Error),
    SVGParseError(usvg::Error),
    PixmapAllocationError,
    ReadStaticPNGError,
    PngEncodeError,
    PngDecodeError(ImageError),
    ImageError(ImageError),
    UnknownLayoutId(u32),
    RegexError(regex::Error),
    BackgroundsNotSupported(String),
}

/// An RGBA image buffer.
pub type ImgBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
/** The main renderer. */
pub struct Renderer<'a> {
    /// The schema that determines the layout of the render and all of its elements.
    schema: Schema,
    layout: SchemaLayout,
    /// Tracks placeholders that have been used.
    used_placeholders: UsedPlaceholders,
    /// The placeholder values to use.
    values: PlaceholderValues,
    /// The usvg options to use.
    usvg_options: &'a Options<'a>,
}

impl<'a> Renderer<'a> {
    /// Creates a new renderer.
    ///
    /// # Arguments
    ///
    /// - `schema` - The schema that determines the layout of the render and all of its elements.
    /// - `values` - The placeholder values to use.
    /// - `options` - The usvg options to use.
    pub fn build(
        schema: Schema,
        layout: SchemaLayout,
        values: PlaceholderValues,
        options: &'a usvg::Options<'a>,
    ) -> Self {
        Renderer {
            schema: schema,
            layout: layout,
            used_placeholders: UsedPlaceholders::new(),
            values,
            usvg_options: options,
        }
    }

    /// Returns the X position with respect to the content box.
    ///
    /// # Arguments
    ///
    /// - `x` - The X position specified in the schema.
    fn get_x(&self, x: i32) -> i64 {
        (self.layout.content_box.raster_x as i32 + x).into()
    }

    /// Returns the Y position with respect to the content box.
    ///
    /// # Arguments
    ///
    /// - `y` - The Y position specified in the schema.
    fn get_y(&self, y: i32) -> i64 {
        (self.layout.content_box.raster_y as i32 + y).into()
    }

    /// Creates a new pixmap for rendering fragments onto.
    fn create_composite_pixmap(&self) -> Result<Pixmap, RenderingError> {
        let raster_size = &self.layout.raster_size;

        let pixmap = Pixmap::new(raster_size.width, raster_size.height)
            .ok_or(RenderingError::PixmapAllocationError)?;

        Ok(pixmap)
    }

    /// Renders an SVG onto a pixmap.
    ///
    /// # Arguments
    ///
    /// - `svg_code` - The SVG code to render.
    /// - `x` - The X position to render at.
    /// - `y` - The Y position to render at.
    /// - `pixmap_mut` - The pixmap to render onto.
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

    /// Converts a pixmap to an RGBA image buffer.
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

    /// Replaces placeholders with values in SVG code.
    ///
    /// # Arguments
    ///
    /// - `schema_placeholders` - The placeholders specified in the schema.
    /// - `placeholder_values` - The placeholder values to use.
    /// - `svg_code` - The SVG code to replace placeholders in.
    /// - `used_placeholders` - The placeholders that have been used.
    /// - `unused_placeholders` - The placeholders that have not been used.
    fn replace_placeholders(
        schema_placeholders: &Vec<String>,
        placeholder_values: &PlaceholderValueMap,
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

    /// Replaces variables with values in SVG code. Must be called after replacing placeholders as
    /// placeholder values may contain variables.
    ///
    /// # Arguments
    ///
    /// - `svg_code` - The SVG code to replace variables in.
    ///
    /// Returns the SVG code with variables replaced.
    fn replace_variables(&self, svg_code: String) -> Result<String, RenderingError> {
        let re = Regex::new(r"\{variable:([a-zA-Z0-9_.]+)\}")
            .map_err(|e| RenderingError::RegexError(e))?;

        let mut updated_svg_code = svg_code.clone();

        for caps in re.captures_iter(&svg_code) {
            let variable = self.schema.get_variable(&caps[1]);
            if let Some(variable) = variable {
                updated_svg_code = updated_svg_code
                    .replace(&format!("{{variable:{}}}", &caps[1]), &variable.value);
            }
        }

        Ok(updated_svg_code)
    }

    /// Renders all specified fragments onto a pixmap. Fragments can be of any fragment type.
    ///
    /// # Arguments
    ///
    /// - `fragments` - The fragments to render.
    /// - `fragments_pixmap_mut` - The pixmap to render onto.
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

            svg_code = self.replace_variables(svg_code)?;

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

    /// Renders all fragments onto a background image.
    fn render_to_background(&mut self, background_img: &mut ImgBuf) -> Result<(), RenderingError> {
        let mut fragments_pixmap = self.create_composite_pixmap()?;
        let mut fragments_pixmap_mut = fragments_pixmap.as_mut();
        self.render_fragments(
            &self.layout.fragments.text.clone(),
            &mut fragments_pixmap_mut,
        )?;
        self.render_fragments(
            &self.layout.fragments.images.clone(),
            &mut fragments_pixmap_mut,
        )?;
        self.render_fragments(
            &self.layout.fragments.shapes.clone(),
            &mut fragments_pixmap_mut,
        )?;

        let fragments_img = Renderer::pixmap_to_png(fragments_pixmap)?;

        overlay(background_img, &fragments_img, 0, 0);

        Ok(())
    }

    /// Utility function for blending two RGBA values.
    ///
    /// # Arguments
    ///
    /// - `src` - The source RGBA value.
    /// - `dst` - The destination RGBA value.
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

    /// Utility function for overlaying two images with a mask.
    ///
    /// - If the mask pixel is not white, the bottom pixel is replaced with the top pixel.
    ///
    /// Otherwise if the mask pixel is white:
    /// - If the top pixel is opaque, the bottom pixel is replaced with the top pixel.
    /// - Otherwise if the top pixel is not completely transparent, the bottom pixel is blended with the top pixel.
    /// - Otherwise the bottom pixel is left unchanged.
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

    /// Loads an RGBA image buffer from a file.
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

    fn resize_background_image(background_img: ImgBuf, size: (u32, u32)) -> ImgBuf {
        let bg_width = background_img.width();
        let bg_height = background_img.height();

        if bg_width == size.0 && bg_height == size.1 {
            return background_img;
        }

        let width_ratio = size.0 as f32 / bg_width as f32;
        let height_ratio = size.1 as f32 / bg_height as f32;

        let ratio = width_ratio.max(height_ratio);

        let new_height = (bg_height as f32 * ratio) as u32;
        let new_width = (bg_width as f32 * ratio) as u32;

        let mut resized = resize(
            &background_img,
            new_width,
            new_height,
            FilterType::CatmullRom,
        );
        if (resized.width(), resized.height()) != size {
            return crop(&mut resized, 0, 0, size.0, size.1).to_image();
        }

        return resized;
    }

    /// Creates a translucent base image by blending the background image with the translucent base image.
    pub fn create_translucent_base(
        &mut self,
        background_img: ImgBuf,
    ) -> Result<ImgBuf, RenderingError> {
        let background_base = match &self.layout.static_base.background {
            Some(background) => background,
            None => {
                return Err(RenderingError::BackgroundsNotSupported(
                    "Background images are not supported by this layout.".to_string(),
                ));
            }
        };

        let translucent_base = self.load_rgba_img_buf(&background_base.translucent)?;
        let mask = self.load_rgba_img_buf(&background_base.mask)?;

        let mut background_img =
            Renderer::resize_background_image(background_img, mask.dimensions());
        // let mut background_img = background_img;

        Renderer::overlay_with_mask(&mut background_img, &translucent_base, &mask);

        Ok(background_img)
    }

    /// Renders the layout to the opaque base image.
    pub fn render_opaque(&mut self) -> Result<ImgBuf, RenderingError> {
        let opaque_base_src = &self.layout.static_base.default;
        let mut opaque_base = self.load_rgba_img_buf(opaque_base_src)?;

        self.render_to_background(&mut opaque_base)?;
        Ok(opaque_base)
    }

    /// Renders the layout to the translucent base image using the specified background image.
    pub fn render_translucent(&mut self, background_img: ImgBuf) -> Result<ImgBuf, RenderingError> {
        let mut static_base = self.create_translucent_base(background_img)?;
        self.render_to_background(&mut static_base)?;
        Ok(static_base)
    }
}
