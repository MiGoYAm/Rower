#![allow(dead_code)]

use serde::{Serialize, Deserialize};
use serde_with::skip_serializing_none;

pub enum Color {
    RGB(u8, u8, u8),
}

impl Color {
    pub const BLACK: Color = Color::RGB(0, 0, 0);
    pub const DARK_BLUE: Color = Color::RGB(0, 0, 42);
    pub const DARK_GREEN: Color = Color::RGB(0, 42, 0);
    pub const DARK_AQUA: Color = Color::RGB(0, 42, 42);
    pub const DARK_RED: Color = Color::RGB(42, 0, 0);
    pub const DARK_PURPLE: Color = Color::RGB(42, 0, 42);
    pub const GOLD: Color = Color::RGB(42, 42, 0);
    pub const GRAY: Color = Color::RGB(42, 42, 42);
    pub const DARK_GRAY: Color = Color::RGB(21, 21, 21);
    pub const BLUE: Color = Color::RGB(21, 21, 63);
    pub const GREEN: Color = Color::RGB(21, 63, 21);
    pub const AQUA: Color = Color::RGB(21, 63, 63);
    pub const RED: Color = Color::RGB(63, 21, 21);
    pub const LIGHT_PURPLE: Color = Color::RGB(63, 21, 63);
    pub const YELLOW: Color = Color::RGB(63, 63, 21);
    pub const WHITE: Color = Color::RGB(63, 63, 63);
}

/* todo

pub enum Type {
    Text(String),
    Translation {
        translate: String,
        with: Option<Vec<Component>>
    },
    Keybind(String)
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match self {
            Type::Text(text) => 
                serializer.serialize_newtype_variant("type", 0, "text", text),
            Type::Translation { translate, with } => {
                let mut state = 
                    serializer.serialize_struct("translation", 2)?;
                state.serialize_field("translate", translate)?;
                state.serialize_field("with", with)?;
                state.end()
            },
            Type::Keybind(key) => 
                serializer.serialize_newtype_variant("type", 1, "key", key),
        }
    }
}

impl<'de> Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        todo!()
    }
}
*/

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Type {
    Text { text: String },
    Translation {
        translate: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        with: Option<Vec<Component>>
    },
    Keybind { keybind: String }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    pub content: Type,

    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub font: Option<String>,
    //pub color: Option<Color>,
    pub insertion: Option<String>,
    pub extra: Option<Vec<Component>>,
}

impl Component {
    pub fn text(text: String) -> Self {
        Self {
            content: Type::Text { text },
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            font: None,
            insertion: None,
            extra: None,
        }
    }

    pub fn content(content: Type) -> Self {
        Self {
            content,
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            font: None,
            insertion: None,
            extra: None,
        }
    }

    pub fn overwrite_extra(&mut self, components: Vec<Component>) {
        self.extra = Some(components);
    }
    
    pub fn append(&mut self, mut components: Vec<Component>) {
        let extra = self.extra.get_or_insert(Vec::new());
        extra.append(&mut components);
    }

    pub fn push(&mut self, component: Component) {
        let extra = self.extra.get_or_insert(Vec::new());
        extra.push(component)
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
