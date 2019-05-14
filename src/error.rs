use handlebars::{RenderError, TemplateFileError};

use crate::git::GitError;

#[derive(Debug)]
pub enum SmeagolError {
    GitError(GitError),
    TemplateFileError(TemplateFileError),
    TemplateRenderError(RenderError),
}
impl std::error::Error for SmeagolError {}
impl std::fmt::Display for SmeagolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &SmeagolError::GitError(ref err) => write!(f, "Git error: {}", err),
            &SmeagolError::TemplateFileError(ref err) => write!(f, "Template file error: {}", err),
            &SmeagolError::TemplateRenderError(ref err) => {
                write!(f, "Template render error: {}", err)
            }
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
