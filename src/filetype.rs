use std::fmt;

use crate::warp_helper::ContentType;
use crate::Path;

#[derive(Debug)]
pub enum Filetype {
    Raw,
    Png,
    Markdown,
}
impl Filetype {
    pub fn is_safe(&self) -> bool {
        match self {
            &Filetype::Markdown => true,
            _ => false,
        }
    }

    pub fn is_raw(&self) -> bool {
        match self {
            &Filetype::Png => true,
            _ => false,
        }
    }
    pub fn is_raw_inline(&self) -> bool {
        match self {
            &Filetype::Png => true,
            _ => panic!("Attempted to check raw inlining for non-raw filetype"),
        }
    }

    pub fn content_type(&self) -> ContentType {
        match self {
            &Filetype::Raw => ContentType::Binary,
            &Filetype::Png => ContentType::Png,
            &Filetype::Markdown => ContentType::Markdown,
        }
    }

    pub fn parse(&self, data: &str) -> Result<String, ParsingError> {
        match self {
            &Filetype::Markdown => self.parse_markdown(data),
            &Filetype::Raw => Ok(data.to_string()),
            _ => panic!("Attempted to parse raw filetype"),
        }
    }

    fn parse_markdown(&self, data: &str) -> Result<String, ParsingError> {
        let mut options = comrak::ComrakOptions::default();
        options.ext_strikethrough = true;
        options.ext_table = true;
        options.ext_tasklist = true;
        Ok(comrak::markdown_to_html(data, &options))
    }
}
impl From<&Path> for Filetype {
    fn from(path: &Path) -> Filetype {
        if let Some(extension) = path.extension() {
            match std::str::from_utf8(&extension) {
                Ok("md") => Filetype::Markdown,
                Ok("png") => Filetype::Png,
                Ok(_) | Err(_) => Filetype::Raw,
            }
        } else {
            Filetype::Raw
        }
    }
}

#[derive(Debug)]
pub enum ParsingError {}
impl std::error::Error for ParsingError {}
impl fmt::Display for ParsingError {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}
