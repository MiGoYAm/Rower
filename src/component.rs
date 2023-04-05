#![allow(dead_code)]

use serde::{Serialize, Deserialize};

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

#[derive(Serialize, Deserialize)]
pub struct TextComponent {
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscated: Option<bool>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<TextComponent>,
}

impl TextComponent {
    pub fn new(s: String) -> Self {
        Self {
            text: s,
            bold: None,
            italic: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            extra: vec![],
        }
    }

    pub fn overwrite_extra(&mut self, components: Vec<TextComponent>) {
        self.extra = components;
    }
    pub fn append(&mut self, mut components: Vec<TextComponent>) {
        self.extra.append(&mut components);
    }
    pub fn push(&mut self, component: TextComponent) {
        self.extra.push(component)
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
