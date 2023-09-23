#![allow(dead_code)]
use serde::{Deserialize, Serialize, Serializer, Deserializer, de::{Visitor, self}};
use serde_with::skip_serializing_none;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    #[serde(serialize_with = "color_serialize", deserialize_with = "color_deserialize")]
    #[serde(untagged)]
    Rgb(u8, u8, u8),
}

fn color_serialize<S>(red: &u8, green: &u8, blue: &u8, s: S) -> Result<S::Ok, S::Error> 
where 
    S: Serializer
{
    s.serialize_str(&format!("#{:02X}{:02X}{:02X}", red, green, blue))
}

fn color_deserialize<'de, D>(d: D) -> Result<(u8, u8, u8), D::Error> 
where 
    D: Deserializer<'de>
{
    d.deserialize_str(ColorVisitor)
}

fn hex<E>(src: &[u8]) -> Result<u8, E>
where
    E: de::Error 
{
    let str = String::from_utf8_lossy(src);
    u8::from_str_radix(&str, 16).map_err(|e| E::custom(e))
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = (u8, u8, u8);

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string color in hex")
    }

    fn visit_str<E>(self, str: &str) -> Result<Self::Value, E>
        where
            E: de::Error
    {
        let bytes = str.as_bytes();
        if str.len() != 7 || bytes[0] != b'#' {
            return Err(E::custom("string is not a color"));
        }

        Ok((hex(&bytes[1..=2])?, hex(&bytes[3..=4])?, hex(&bytes[5..=6])?))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Text(String),
    Keybind(String),
    #[serde(untagged)]
    Translation {
        translate: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        with: Option<Vec<Component>>,
    },
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct Component {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub font: Option<String>,
    pub color: Option<Color>,
    pub insertion: Option<String>,
    pub extra: Option<Vec<Component>>,
    #[serde(flatten)]
    pub content: Type,
}

impl Component {
    pub const fn content(content: Type) -> Self {
        Self {
            content,
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            font: None,
            color: None,
            insertion: None,
            extra: None,
        }
    }

    pub fn text(text: &str) -> Self {
        Self::content(Type::Text(text.to_owned()))
    }
    
    pub fn append(&mut self, mut components: Vec<Component>) {
        self.extra.get_or_insert(Vec::new()).append(&mut components);
    }

    pub fn push(&mut self, component: Component) {
        self.extra.get_or_insert(Vec::new()).push(component)
    }

    pub fn bold(&mut self, b: bool) {
        self.bold = Some(b);
    }

    pub fn underlined(&mut self, b: bool) {
        self.underlined = Some(b);
    }

    pub fn strikethrough(&mut self, b: bool) {
        self.strikethrough = Some(b);
    }

    pub fn obfuscated(&mut self, b: bool) {
        self.obfuscated = Some(b);
    }
}
