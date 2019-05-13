use crate::git::GitError;

#[derive(Debug)]
pub enum SmeagolError {
    GitError(GitError),
}
impl std::error::Error for SmeagolError {}
impl std::fmt::Display for SmeagolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &SmeagolError::GitError(ref err) => write!(f, "Git error: {}", err),
        }
    }
}
impl From<GitError> for SmeagolError {
    fn from(err: GitError) -> Self {
        SmeagolError::GitError(err)
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
