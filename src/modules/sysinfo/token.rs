use crate::clients::sysinfo::{Function, Prefix, TokenType};

#[derive(Debug, Clone)]
pub struct Token {
    pub token: TokenType,
    pub function: Function,
    pub prefix: Prefix,
    pub formatting: Formatting,
}

#[derive(Debug, Clone)]
pub enum Part {
    Static(String),
    Token(Token),
}

#[derive(Debug, Clone, Copy)]
pub struct Formatting {
    pub width: usize,
    pub fill: char,
    pub align: Alignment,
    pub precision: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}
