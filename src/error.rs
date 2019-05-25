use handlebars::{RenderError, TemplateFileError};

use serde_json::Error as JsonError;

use std::fmt;

use crate::config::ConfigError;
use crate::filetype::ParsingError;
use crate::git::GitError;

#[derive(Debug)]
pub enum SmeagolError {
    Git(GitError),
    Config(ConfigError),
    TemplateFile(TemplateFileError),
    TemplateRender(RenderError),
    SerdeJson(JsonError),
    Parsing(ParsingError),
}
impl std::error::Error for SmeagolError {}
impl fmt::Display for SmeagolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &SmeagolError::Git(ref err) => write!(f, "Git error: {}", err),
            &SmeagolError::Config(ref err) => write!(f, "Config error: {}", err),
            &SmeagolError::TemplateFile(ref err) => write!(f, "Template file error: {}", err),
            &SmeagolError::TemplateRender(ref err) => write!(f, "Template render error: {}", err),
            &SmeagolError::SerdeJson(ref err) => write!(f, "Json error: {}", err),
            &SmeagolError::Parsing(ref err) => write!(f, "Parsing error: {}", err),
        }
    }
}
impl From<GitError> for SmeagolError {
    fn from(err: GitError) -> Self {
        SmeagolError::Git(err)
    }
}
impl From<ConfigError> for SmeagolError {
    fn from(err: ConfigError) -> Self {
        SmeagolError::Config(err)
    }
}
impl From<TemplateFileError> for SmeagolError {
    fn from(err: TemplateFileError) -> Self {
        SmeagolError::TemplateFile(err)
    }
}
impl From<RenderError> for SmeagolError {
    fn from(err: RenderError) -> Self {
        SmeagolError::TemplateRender(err)
    }
}
impl From<JsonError> for SmeagolError {
    fn from(err: JsonError) -> Self {
        SmeagolError::SerdeJson(err)
    }
}
impl From<ParsingError> for SmeagolError {
    fn from(err: ParsingError) -> Self {
        SmeagolError::Parsing(err)
    }
}

impl From<SmeagolError> for warp::reject::Rejection {
    fn from(err: SmeagolError) -> warp::reject::Rejection {
        warp::reject::custom(err)
    }
}
impl From<GitError> for warp::reject::Rejection {
    fn from(err: GitError) -> warp::reject::Rejection {
        warp::reject::custom(SmeagolError::from(err))
    }
}
