use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD as STANDARD_ENGINE};
use fractyl_renderer::http::AxumRenderingServer;
use fractyl_renderer::{
    render::{PlaceholderValues, Renderer},
    schema,
};
use log::info;

#[tokio::main]
async fn main() {
    colog::init();
    rendering_server().await;
}

#[allow(unused)]
async fn rendering_server() {
    let mut server = AxumRenderingServer::new().add_renderer(
        schema::load_schema_from_file("./export/schema.json").unwrap(),
        "/bedwars",
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    info!("Server running on http://localhost:3001");

    server.serve(listener).await.unwrap();
}

#[allow(unused)]
async fn render() {
    let schema = schema::load_schema_from_file("./export/schema.json").unwrap();

    let mut text_placeholders: HashMap<&str, &str> = HashMap::new();
    text_placeholders.insert("stat_wins#text", "3,895");
    text_placeholders.insert("stat_losses#text", "3,065");
    text_placeholders.insert("stat_wlr#text", "1.27");

    text_placeholders.insert("stat_final_kills#text", "11,605");
    text_placeholders.insert("stat_final_deaths#text", "3,132");
    text_placeholders.insert("stat_fkdr#text", "3.71");

    text_placeholders.insert("stat_beds_broken#text", "4,871");
    text_placeholders.insert("stat_beds_lost#text", "3,505");
    text_placeholders.insert("stat_bblr#text", "1.39");

    text_placeholders.insert("stat_kills#text", "16,219");
    text_placeholders.insert("stat_deaths#text", "25,414");
    text_placeholders.insert("stat_kdr#text", "0.64");

    text_placeholders.insert("stat_games_played#text", "6,969");
    text_placeholders.insert("stat_most_played#text", "Fours");

    text_placeholders.insert("level_current#text", "[486✫]");
    text_placeholders.insert("level_current#fill", "#00AA00");
    text_placeholders.insert("level_next#text", "[487✫]");
    text_placeholders.insert("level_next#fill", "#00AA00");

    text_placeholders.insert("gamemode#text", "Overall");
    text_placeholders.insert("bedwars_tokens#text", "327,152");
    text_placeholders.insert("slumber_tickets#text", "103");

    text_placeholders.insert("footer_info#text.0", "statalytics.net • ");
    text_placeholders.insert("footer_info#text.1", "Saturday 10th January, 2026");

    text_placeholders.insert("displayname#text.0", "[MVP");
    text_placeholders.insert("displayname#text.1", "+");
    text_placeholders.insert("displayname#text.2", "] Lucism");

    text_placeholders.insert("xp_progress#text", "1,666 / 5,000 xp");

    let mut shape_placeholders: HashMap<&str, &str> = HashMap::new();
    shape_placeholders.insert("progress_bar#width", "500");
    shape_placeholders.insert("progress_bar#gradientStop.0", "#004400");
    shape_placeholders.insert("progress_bar#gradientStop.1", "#00AA00");

    let mut image_placeholders: HashMap<&str, &str> = HashMap::new();

    let res = reqwest::get("https://vzge.me/bust/lucism").await.unwrap();
    let skin_bytes = res.bytes().await.unwrap();

    let href = format!(
        "data:image/png;base64,{}",
        STANDARD_ENGINE.encode(skin_bytes)
    );

    image_placeholders.insert("skin_model#href", &href);

    let mut renderer = Renderer::build(
        schema,
        PlaceholderValues {
            text: text_placeholders
                .iter()
                .map(|m| (m.0.to_string(), m.1.to_string()))
                .collect(),
            shapes: shape_placeholders
                .iter()
                .map(|m| (m.0.to_string(), m.1.to_string()))
                .collect(),
            images: image_placeholders
                .iter()
                .map(|m| (m.0.to_string(), m.1.to_string()))
                .collect(),
        },
    );

    let background_img = image::open("./backgrounds/landscape.png")
        .unwrap()
        .to_rgba8();

    let output = renderer.render_translucent(background_img).unwrap();
    output.save("./output/debug.png").unwrap();
}
