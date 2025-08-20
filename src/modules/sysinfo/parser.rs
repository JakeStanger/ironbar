use crate::clients::sysinfo::{Function, Prefix, TokenType};
use crate::modules::sysinfo::token::{Alignment, Formatting, Part, Token};
use color_eyre::{Report, Result};
use std::iter::Peekable;
use std::str::{Chars, FromStr};

impl Function {
    pub(crate) fn default_for(token_type: TokenType) -> Self {
        match token_type {
            TokenType::CpuFrequency
            | TokenType::CpuPercent
            | TokenType::TempC
            | TokenType::DiskPercent => Self::Mean,
            TokenType::DiskFree
            | TokenType::DiskTotal
            | TokenType::DiskUsed
            | TokenType::DiskRead
            | TokenType::DiskWrite
            | TokenType::NetDown
            | TokenType::NetUp => Self::Sum,
            _ => Self::None,
        }
    }
}

impl Prefix {
    pub(crate) fn default_for(token_type: TokenType) -> Self {
        match token_type {
            TokenType::CpuFrequency
            | TokenType::MemoryFree
            | TokenType::MemoryAvailable
            | TokenType::MemoryTotal
            | TokenType::MemoryUsed
            | TokenType::SwapFree
            | TokenType::SwapTotal
            | TokenType::SwapUsed
            | TokenType::DiskFree
            | TokenType::DiskTotal
            | TokenType::DiskUsed => Self::Giga,
            TokenType::DiskRead | TokenType::DiskWrite => Self::Mega,
            TokenType::NetDown | TokenType::NetUp => Self::MegaBit,
            _ => Self::None,
        }
    }
}

impl FromStr for Prefix {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "k" => Ok(Prefix::Kilo),
            "M" => Ok(Prefix::Mega),
            "G" => Ok(Prefix::Giga),
            "T" => Ok(Prefix::Tera),
            "P" => Ok(Prefix::Peta),

            "ki" => Ok(Prefix::Kibi),
            "Mi" => Ok(Prefix::Mebi),
            "Gi" => Ok(Prefix::Gibi),
            "Ti" => Ok(Prefix::Tebi),
            "Pi" => Ok(Prefix::Pebi),

            "kb" => Ok(Prefix::KiloBit),
            "Mb" => Ok(Prefix::MegaBit),
            "Gb" => Ok(Prefix::GigaBit),

            _ => Err(Report::msg(format!("invalid prefix: {s}"))),
        }
    }
}

impl TryFrom<char> for Alignment {
    type Error = Report;

    fn try_from(value: char) -> Result<Self> {
        match value {
            '<' => Ok(Self::Left),
            '^' => Ok(Self::Center),
            '>' => Ok(Self::Right),
            _ => Err(Report::msg(format!("Unknown alignment: {value}"))),
        }
    }
}

impl Formatting {
    fn default_for(token_type: TokenType) -> Self {
        match token_type {
            TokenType::CpuFrequency
            | TokenType::LoadAverage1
            | TokenType::LoadAverage5
            | TokenType::LoadAverage15 => Self {
                width: 0,
                fill: '0',
                align: Alignment::default(),
                precision: 2,
            },
            TokenType::CpuPercent => Self {
                width: 2,
                fill: '0',
                align: Alignment::default(),
                precision: 0,
            },
            TokenType::MemoryFree
            | TokenType::MemoryAvailable
            | TokenType::MemoryTotal
            | TokenType::MemoryUsed
            | TokenType::MemoryPercent
            | TokenType::SwapFree
            | TokenType::SwapTotal
            | TokenType::SwapUsed
            | TokenType::SwapPercent => Self {
                width: 4,
                fill: '0',
                align: Alignment::default(),
                precision: 1,
            },
            _ => Self {
                width: 0,
                fill: '0',
                align: Alignment::default(),
                precision: 0,
            },
        }
    }
}

pub fn parse_input(input: &str) -> Result<Vec<Part>> {
    let mut tokens = vec![];

    let mut chars = input.chars().peekable();

    let mut next_char = chars.peek().copied();
    while let Some(char) = next_char {
        let token = if char == '{' {
            chars.next();
            parse_dynamic(&mut chars)?
        } else {
            parse_static(&mut chars)
        };

        tokens.push(token);
        next_char = chars.peek().copied();
    }

    Ok(tokens)
}

fn parse_static(chars: &mut Peekable<Chars>) -> Part {
    let mut str = String::new();

    let mut next_char = chars.next_if(|&c| c != '{');
    while let Some(char) = next_char {
        if char == '{' {
            break;
        }

        str.push(char);
        next_char = chars.next_if(|&c| c != '{');
    }

    Part::Static(str)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum DynamicMode {
    Token,
    Name,
    Prefix,
}

fn parse_dynamic(chars: &mut Peekable<Chars>) -> Result<Part> {
    let mut mode = DynamicMode::Token;

    let mut token_str = String::new();
    let mut func_str = String::new();
    let mut prefix_str = String::new();

    // we don't want to peek here as that would be the same char as the outer loop
    let mut next_char = chars.next();
    while let Some(char) = next_char {
        match char {
            '}' | ':' => break,
            '@' => mode = DynamicMode::Name,
            '#' => mode = DynamicMode::Prefix,
            _ => match mode {
                DynamicMode::Token => token_str.push(char),
                DynamicMode::Name => func_str.push(char),
                DynamicMode::Prefix => prefix_str.push(char),
            },
        }

        next_char = chars.next();
    }

    let token_type = token_str.parse()?;
    let mut formatting = Formatting::default_for(token_type);

    if next_char == Some(':') {
        formatting = parse_formatting(chars, formatting)?;
    }

    let token = Token {
        token: token_type,
        function: func_str
            .parse()
            .unwrap_or_else(|()| Function::default_for(token_type)),
        prefix: prefix_str
            .parse()
            .unwrap_or_else(|_| Prefix::default_for(token_type)),
        formatting,
    };

    Ok(Part::Token(token))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum FormattingMode {
    WidthFillAlign,
    Precision,
}

fn parse_formatting(chars: &mut Peekable<Chars>, mut formatting: Formatting) -> Result<Formatting> {
    let mut width_string = String::new();
    let mut precision_string = String::new();

    let mut mode = FormattingMode::WidthFillAlign;

    let mut next_char = chars.next();
    while let Some(char) = next_char {
        match (char, mode) {
            ('}', _) => break,
            ('.', _) => mode = FormattingMode::Precision,
            (_, FormattingMode::Precision) => precision_string.push(char),
            ('1'..='9', FormattingMode::WidthFillAlign) => width_string.push(char),
            ('<' | '^' | '>', FormattingMode::WidthFillAlign) => {
                formatting.align = Alignment::try_from(char)?;
            }
            (_, FormattingMode::WidthFillAlign) => formatting.fill = char,
        }

        next_char = chars.next();
    }

    if !width_string.is_empty() {
        formatting.width = width_string.parse()?;
    }

    if !precision_string.is_empty() {
        formatting.precision = precision_string.parse()?;
    }

    Ok(formatting)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_only() {
        let tokens = parse_input("hello world").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Part::Static(str) if str == "hello world"));
    }

    #[test]
    fn basic() {
        let tokens = parse_input("{cpu_frequency}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
    }

    #[test]
    fn named() {
        let tokens = parse_input("{cpu_frequency@cpu0}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert!(matches!(&token.function, Function::Name(n) if n == "cpu0"));
    }

    #[test]
    fn conversion() {
        let tokens = parse_input("{cpu_frequency#G}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert_eq!(token.prefix, Prefix::Giga);
    }

    #[test]
    fn formatting_basic() {
        let tokens = parse_input("{cpu_frequency:.2}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert_eq!(token.formatting.precision, 2);
    }

    #[test]
    fn formatting_complex() {
        let tokens = parse_input("{cpu_frequency:0<5.2}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert_eq!(token.formatting.fill, '0');
        assert_eq!(token.formatting.align, Alignment::Left);
        assert_eq!(token.formatting.width, 5);
        assert_eq!(token.formatting.precision, 2);
    }

    #[test]
    fn complex() {
        let tokens = parse_input("{cpu_frequency@cpu0#G:.2}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 1);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert!(matches!(&token.function, Function::Name(n) if n == "cpu0"));
        assert_eq!(token.prefix, Prefix::Giga);
        assert_eq!(token.formatting.precision, 2);
    }

    #[test]
    fn static_then_token() {
        let tokens = parse_input("Freq: {cpu_frequency#G:.2}").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 2);

        assert!(matches!(&tokens[0], Part::Static(str) if str == "Freq: "));

        assert!(matches!(&tokens[1], Part::Token(_)));
        let Part::Token(token) = tokens.get(1).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert_eq!(token.formatting.precision, 2);
    }

    #[test]
    fn token_then_static() {
        let tokens = parse_input("{cpu_frequency#G:.2} GHz").unwrap();
        println!("{tokens:?}");

        assert_eq!(tokens.len(), 2);

        assert!(matches!(&tokens[0], Part::Token(_)));
        let Part::Token(token) = tokens.get(0).unwrap() else {
            return;
        };

        assert_eq!(token.token, TokenType::CpuFrequency);
        assert_eq!(token.formatting.precision, 2);

        assert!(matches!(&tokens[1], Part::Static(str) if str == " GHz"));
    }
}
