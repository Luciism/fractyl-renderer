use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD as STANDARD_ENGINE};
use fractyl_renderer::{render::{PlaceholderValues, Renderer}, schema};




#[tokio::main]
async fn main() {
    colog::init();

    let schema = schema::load_schema_from_file("./export/schema.json").unwrap();

    let s = |string: &str| {
        string.to_string()
    };
    
    let mut text_placeholders: HashMap<&str, String> = HashMap::new();
    text_placeholders.insert("stat_wins#text", s("3,895"));
    text_placeholders.insert("stat_losses#text", s("3,065"));
    text_placeholders.insert("stat_wlr#text", s("1.27"));

    text_placeholders.insert("stat_final_kills#text", s("11,605"));
    text_placeholders.insert("stat_final_deaths#text", s("3,132"));
    text_placeholders.insert("stat_fkdr#text", s("3.71"));

    text_placeholders.insert("stat_beds_broken#text", s("4,871"));
    text_placeholders.insert("stat_beds_lost#text", s("3,505"));
    text_placeholders.insert("stat_bblr#text", s("1.39"));

    text_placeholders.insert("stat_kills#text", s("16,219"));
    text_placeholders.insert("stat_deaths#text", s("25,414"));
    text_placeholders.insert("stat_kdr#text", s("0.64"));

    text_placeholders.insert("stat_games_played#text", s("6,969"));
    text_placeholders.insert("stat_most_played#text", s("Fours"));

    text_placeholders.insert("level_current#text", s("[486✫]"));
    text_placeholders.insert("level_current#fill", s("#00AA00"));
    text_placeholders.insert("level_next#text", s("[487✫]"));
    text_placeholders.insert("level_next#fill", s("#00AA00"));

    text_placeholders.insert("gamemode#text", s("Overall"));
    text_placeholders.insert("bedwars_tokens#text", s("327,152"));
    text_placeholders.insert("slumber_tickets#text", s("103"));

    text_placeholders.insert("footer_info#text.0", s("statalytics.net • "));
    text_placeholders.insert("footer_info#text.1", s("Saturday 10th January, 2026"));

    text_placeholders.insert("displayname#text.0", s("[MVP"));
    text_placeholders.insert("displayname#text.1", s("+"));
    text_placeholders.insert("displayname#text.2", s("] Lucism"));

    text_placeholders.insert("xp_progress#text", s("1,666 / 5,000 xp"));

    let mut shape_placeholders: HashMap<&str, String> = HashMap::new();
    shape_placeholders.insert("progress_bar#width", s("500"));
    shape_placeholders.insert("progress_bar#gradientStop.0", s("#004400"));
    shape_placeholders.insert("progress_bar#gradientStop.1", s("#00AA00"));

    let mut image_placeholders: HashMap<&str, String> = HashMap::new();

    let res = reqwest::get("https://vzge.me/bust/lucism").await.unwrap();
    let skin_bytes = res.bytes().await.unwrap();
    
    let href = format!("data:image/png;base64,{}", STANDARD_ENGINE.encode(skin_bytes));

    image_placeholders.insert("skin_model#href", href);

    let mut renderer = Renderer::build(schema, PlaceholderValues {
        text: text_placeholders,
        shapes: shape_placeholders,
        images: image_placeholders,
    });

    let background_img = image::open("./backgrounds/landscape.png").unwrap().to_rgba8();

    let output = renderer.render_translucent(background_img).unwrap();
    output.save("./output/debug.png").unwrap();
}
