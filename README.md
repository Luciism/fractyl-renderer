# Fractyl Renderer

Fractyl is a system for creating game statistic cards. It uses Figma as a design tool and generates SVG templates that can be filled in with data at render time.

<!-- For more information on Fractyl, see [TODO](https://todo). -->

## Rendering

The renderer can either be used directly through the `Renderer` struct or by creating a REST API using the `AxumRenderingServer` struct.

### Direct Usage

```rust
use image;
use fractyl_renderer::render::{Renderer, PlaceholderValues, TextSpan};
use fracyl_renderer::schema::load_schema_from_file;

// Load the automatically generated template schema
let schema = load_schema_from_file("exports/example/schema.json").unwrap();

// Create a mapping of placeholder values to fill in
let mut shape_values = HashMap::new();
shape_values.insert("progress_bar#width", "120");
shape_values.insert("progress_bar#fill", "#00FF00");

let mut image_values = HashMap::new();
shape_values.insert("player_model#href", "data:image/png;base64,...");

// Unset styles will fallback to parent styles
let mut text_values = HashMap::new();
text_values.insert("stat_wins#text", TextSpan {value: "5", fill: None, font_size: None, font_weight: None, font_family: None});


let values = PlaceholderValues {
    shapes: shapes,
    images: images,
    text: text
};

let mut options = usvg::Options::default();
options.fontdb_mut().load_fonts_dir("./fonts/");

let renderer = Renderer::new(schema, values, options);

// Render regular template
renderer.render_opaque().unwrap();

// Render translucent template with background image
renderer.render_translucent(
    image::open("path/to/image.png").unwrap().to_rgba8()
).unwrap();
```

### REST API

Setup rendering server:

```rust
use fractyl_renderer::http::AxumRenderingServer;

#[tokio::main]
async fn main() {
    // Manually specify routing
    let server = AxumRenderingServer::new().add_renderer(
        schema::load_schema_from_file("./exports/example/schema.json").unwrap(),
        "/example1",
    ).add_renderer(
        schema::load_schema_from_file("./exports/example2/schema.json").unwrap(),
        "/example2",
    );

    // Or automatically discover templates (this will use the template directory name as the route path)
    let server = AxumRenderingServer::new()
        .discover_templates()
        .unwrap();

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    server.serve(listener).await.unwrap();
}
```

Then make requests to the HTTP server (127.0.0.1:3001 in this example):
```py
placeholder_values = {
    "text": {
        "stat_wins#text": {"value": "5"}
    },
    "shapes": {
        "progress_bar#width": "120",
        "progress_bar#fill": "#00FF00"
    },
    "images": {
        "player_model#href": "data:image/png;base64,..."
    }
}

async with ClientSession(timeout=ClientTimeout(total=10)) as session:
    # Create form data with placeholder values and optionally background image
    data = aiohttp.FormData()
    data.add_field(
        "placeholder_values",
        json.dumps(placeholder_values).encode("utf-8"),
        filename="blob",
        content_type="application/json",
    )

    # Remove this field to not use a background image
    data.add_field(
        "background_image",
        open("path/to/image.png", "rb").read(),
        filename="blob",
        content_type="image/png",
    )

    res = await session.post(f"127.0.0.1:3001/example", data=data)
    res.raise_for_status()

    # Rendered output
    render_bytes = await res.content.read()
```