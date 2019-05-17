use handlebars::{RenderError, TemplateFileError};

use serde_json::Error as JsonError;

use std::fmt;

use crate::filetype::ParsingError;
use crate::git::GitError;

#[derive(Debug)]
pub enum SmeagolError {
    GitError(GitError),
    TemplateFileError(TemplateFileError),
    TemplateRenderError(RenderError),
    SerdeJsonError(JsonError),
    ParsingError(ParsingError),
}
impl std::error::Error for SmeagolError {}
impl fmt::Display for SmeagolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &SmeagolError::GitError(ref err) => write!(f, "Git error: {}", err),
            &SmeagolError::TemplateFileError(ref err) => write!(f, "Template file error: {}", err),
            &SmeagolError::TemplateRenderError(ref err) => {
                write!(f, "Template render error: {}", err)
            }
            &SmeagolError::SerdeJsonError(ref err) => write!(f, "Json error: {}", err),
            &SmeagolError::ParsingError(ref err) => write!(f, "Parsing error: {}", err),
        }
    }
}
impl From<GitError> for SmeagolError {
    fn from(err: GitError) -> Self {
        SmeagolError::GitError(err)
    }
}
impl From<TemplateFileError> for SmeagolError {
    fn from(err: TemplateFileError) -> Self {
        SmeagolError::TemplateFileError(err)
    }
}
impl From<RenderError> for SmeagolError {
    fn from(err: RenderError) -> Self {
        SmeagolError::TemplateRenderError(err)
    }
}
impl From<JsonError> for SmeagolError {
    fn from(err: JsonError) -> Self {
        SmeagolError::SerdeJsonError(err)
    }
}
impl From<ParsingError> for SmeagolError {
    fn from(err: ParsingError) -> Self {
        SmeagolError::ParsingError(err)
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
