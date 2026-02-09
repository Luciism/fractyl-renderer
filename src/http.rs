use std::{io::Cursor, path::Path, sync::Mutex, time};

use axum::{
    Router,
    body::Body,
    extract::DefaultBodyLimit,
    http::{self, Response, StatusCode},
    routing::post,
};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use image::ImageFormat;
use log::info;
use resvg::usvg;
use tokio::net::TcpListener;

use crate::{
    render::Renderer,
    placeholders::PlaceholderValues,
    schema::{self, Schema},
};

const DEFAULT_EXPORT_DIR: &str = "templates";


lazy_static::lazy_static! {
    pub static ref USVG_OPTIONS: Mutex<usvg::Options<'static>> = Mutex::new(usvg::Options::default());
}

#[derive(Debug, TryFromMultipart)]
/// Form data for creating a new render.
struct CreateRenderData {
    #[form_data(limit = "10MB")]
    /// The background image to use for the render.
    background_image: Option<FieldData<axum::body::Bytes>>,

    /// The placeholder values to use for the render.
    pub placeholder_values: FieldData<String>,
}

#[derive(Debug)]
/// Errors that can occur during template discovery.
pub enum DiscoveryError {
    /// An I/O error occurred.
    IoError(std::io::Error),
    /// A schema error occurred.
    SchemaError(schema::SchemaError),
}

impl From<std::io::Error> for DiscoveryError {
    fn from(value: std::io::Error) -> Self {
        DiscoveryError::IoError(value)
    }
}

impl From<schema::SchemaError> for DiscoveryError {
    fn from(value: schema::SchemaError) -> Self {
        DiscoveryError::SchemaError(value)
    }
}

/// An HTTP server that can be used to render templates with the provided placeholder values.
pub struct AxumRenderingServer {
    /// The router for the HTTP server.
    app_router: Router,
}

impl AxumRenderingServer {
    /// Creates a new AxumRenderingServer.
    pub fn new() -> Self {
        let mut options = USVG_OPTIONS.lock().unwrap();
        options.fontdb_mut().load_fonts_dir("./fonts/");

        AxumRenderingServer {
            app_router: Router::new(),
        }
    }

    /// Starts the AxumRenderingServer.
    ///
    /// # Arguments
    ///
    /// - `listener` - The listener to use for the HTTP server.
    pub async fn serve(self, listener: TcpListener) -> Result<(), std::io::Error> {
        axum::serve(listener, self.app_router).await
    }

    /// Returns the router for the AxumRenderingServer.
    pub fn router(&self) -> &Router {
        &self.app_router
    }

    /// Discovers templates in the default templates directory and adds them to the AxumRenderingServer.
    pub fn discover_templates(mut self) -> Result<Self, DiscoveryError> {
        let templates_dir = Path::new(DEFAULT_EXPORT_DIR);

        if templates_dir.try_exists()? && templates_dir.is_dir() {
            for entry in templates_dir.read_dir()? {
                let entry = entry?;

                if !entry.path().is_dir() {
                    continue;
                }

                let mut schema_path = entry.path();
                schema_path.push("schema.json");

                let dir_filename = entry.file_name();
                let dirname = dir_filename
                    .to_str()
                    .ok_or(std::io::Error::other("Failed to read dirname"))?;

                let schema =
                    schema::load_schema_from_file(&schema_path.to_string_lossy().to_string())?;

                self = self.add_renderer(schema, &format!("/{dirname}"));
            }
        }

        Ok(self)
    }

    /// Adds a new renderer route to the rendering server.
    ///
    /// # Arguments
    ///
    /// - `schema` - The schema to use for the renderer.
    /// - `route_path` - The route path to use for the renderer.
    pub fn add_renderer(mut self, schema: Schema, route_path: &str) -> Self {
        let schema_file = schema.schema_file.clone();

        self.app_router = self.app_router.route(
            route_path,
            post(
                async |TypedMultipart(form): TypedMultipart<CreateRenderData>| -> Result<Response<Body>, StatusCode> {
                    let placeholder_values: PlaceholderValues =
                        serde_json::from_str(&form.placeholder_values.contents)
                            .map_err(|_| StatusCode::BAD_REQUEST)?;

                    let options = USVG_OPTIONS.lock().map_err(|e| {
                        log::error!("Failed to acquire options: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                    let mut renderer = Renderer::build(schema, placeholder_values, &options);

                    let start_time = time::Instant::now();
                    let output = match form.background_image {
                        None => renderer.render_opaque().map_err(|e| {
                            log::error!("Failed to add renderer: {e:#?}");
                            StatusCode::INTERNAL_SERVER_ERROR})?,
                        Some(background_image) => {
                            match background_image.metadata.content_type {
                                Some(content_type) => {
                                    if content_type != "image/png" {
                                        return Err(StatusCode::BAD_REQUEST);
                                    }
                                },
                                None => return Err(StatusCode::BAD_REQUEST)
                            }

                            let cursor = Cursor::new(background_image.contents);
                            let image = image::load(cursor, ImageFormat::Png).map_err(|_| StatusCode::BAD_REQUEST)?.to_rgba8();
                            renderer.render_translucent(image).map_err(|e| {
                                log::error!("Rendering failed: {e:#?}");
                                StatusCode::INTERNAL_SERVER_ERROR})?
                        }
                    };

                    let render_time =  time::Instant::now() - start_time;
                    let start_time = time::Instant::now();

                    let mut output_buffer = Vec::new();
                    output.write_to(&mut Cursor::new(&mut output_buffer), ImageFormat::Png).map_err(|e| {
                        log::error!("Failed to write PNG image to buffer: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR})?;

                    let write_time = time::Instant::now() - start_time;

                    info!("Render Time: {}ms", render_time.as_millis());
                    info!("Write Time: {}ms", write_time.as_millis());

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(http::header::CONTENT_TYPE, "image/png")
                        .body(Body::from(output_buffer))
                        .map_err(|e| {
                            log::error!("Failed to send response: {e:#?}");
                            StatusCode::INTERNAL_SERVER_ERROR})?)
                },
            ),
        ).layer(DefaultBodyLimit::max(10*1024*1025));

        info!("Registered {route_path} for schema at '{schema_file}'");
        self
    }
}
