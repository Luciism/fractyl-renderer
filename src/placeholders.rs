use std::collections::HashMap;

use serde::Deserialize;

pub type ExpectedPlaceholders = Vec<String>;
pub type PlaceholderValueMap = HashMap<String, String>;
pub type TextPlaceholderValueMap = HashMap<String, TextPlaceholderValue>;

#[derive(Deserialize, Debug, Clone)]
/// Placeholder values.
pub struct PlaceholderValues {
    /// Text placeholder values.
    pub text: TextPlaceholderValueMap,
    /// Image placeholder values.
    pub images: PlaceholderValueMap,
    /// Shape placeholder values.
    pub shapes: PlaceholderValueMap,
}

impl PlaceholderValues {
    /// Converts the text placeholder values to a map.
    pub fn text(&self) -> PlaceholderValueMap {
        let mut map = HashMap::new();

        for (id, value) in &self.text {
            let output_value = match value {
                TextPlaceholderValue::MultiTSpan(spans) => {
                    let tspans: Vec<String> = spans.iter().map(|span| span.to_tspan()).collect();
                    tspans.join("")
                }
                TextPlaceholderValue::SingleTSpan(span) => span.to_tspan(),
                TextPlaceholderValue::String(str_val) => str_val.to_string()
            };
            map.insert(id.clone(), output_value);
        }

        map
    }

    /// Returns the image placeholder values.
    pub fn images(&self) -> PlaceholderValueMap {
        self.images.clone()
    }

    /// Returns the shape placeholder values.
    pub fn shapes(&self) -> PlaceholderValueMap {
        self.shapes.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TextPlaceholderValue {
    MultiTSpan(Vec<TextSpan>),
    SingleTSpan(TextSpan),
    String(String)
}

#[derive(Deserialize, Debug, Clone)]
/// Text placeholder subvalues.
pub struct TextSpan {
    /// The text value.
    pub value: String,
    /// Optional: The fill color.
    pub fill: Option<String>,
    /// Optional: The font size.
    pub font_size: Option<f32>,
    /// Optional: The font weight.
    pub font_weight: Option<u32>,
    /// Optional: The font family.
    pub font_family: Option<String>,
}

impl TextSpan {
    /// Escapes special characters that could be interpreted as XML.
    fn escaped_value(&self) -> String {
        self.value.replace("&", "&amp;").replace(">", "&gt;").replace("<", "&lt;")
    }

    /// Converts the text span to an XML tspan element string.
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
/// Track placeholders that have been used. This is used to generate warnings when placeholders are not used.
pub struct UsedPlaceholders {
    /// Text placeholders that have been used.
    pub text: Vec<String>,
    /// Image placeholders that have been used.
    pub images: Vec<String>,
    /// Shape placeholders that have been used.
    pub shapes: Vec<String>,
}

impl UsedPlaceholders {
    /// Creates a new UsedPlaceholders struct.
    pub fn new() -> Self {
        UsedPlaceholders {
            text: vec![],
            images: vec![],
            shapes: vec![],
        }
    }
}

