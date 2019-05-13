use std::path::Path;

use git2::{Commit, ErrorCode, Object, Repository, RepositoryInitOptions, Signature};

pub struct GitRepository {
    repo: Repository,
}
impl GitRepository {
    pub fn new<T: AsRef<Path>>(dir: T) -> Result<GitRepository, GitError> {
        Ok(GitRepository {
            repo: Repository::init_opts(
                dir,
                RepositoryInitOptions::new()
                    .bare(true)
                    .mkdir(true)
                    .mkpath(false),
            )?,
        })
    }

    fn head<'repo>(&'repo self) -> Result<Commit<'repo>, GitError> {
        let head_ref = match self.repo.head() {
            Ok(head_ref) => head_ref,
            Err(err) => {
                if err.code() == ErrorCode::UnbornBranch {
                    let signature = Signature::now("smeagol", "smeagol@smeagol")?;
                    let tree_oid = self.repo.treebuilder(None)?.write()?;
                    let tree = self.repo.find_tree(tree_oid)?;
                    self.repo.commit(
                        Some("HEAD"),
                        &signature,
                        &signature,
                        "Root commit",
                        &tree,
                        &[],
                    )?;
                    // We just created the head, therefore we can unwrap
                    self.repo.head().unwrap()
                } else {
                    return Err(err.into());
                }
            }
        };

        // I assume the reference given by head() is valid and a commit.
        let head_oid = head_ref.target().unwrap();
        Ok(self.repo.find_commit(head_oid).unwrap())
    }

    pub fn item<'repo>(&'repo self, path: Vec<Vec<u8>>) -> Result<GitItem<'repo>, GitError> {
        Ok(GitItem {
            repo: self,
            path: path,
        })
    }
}

pub struct GitItem<'repo> {
    repo: &'repo GitRepository,
    path: Vec<Vec<u8>>,
}
impl<'repo> GitItem<'repo> {
    fn object(&self) -> Result<Object<'repo>, GitError> {
        // TODO cache object

        if self.path.len() == 0 {
            return Ok(self.repo.head()?.tree()?.into_object());
        }

        let tree = if self.path.len() == 1 {
            self.repo.head()?.tree()?
        } else {
            let parent_item = GitItem {
                repo: self.repo,
                path: self.path[..self.path.len() - 1].to_vec(),
            };

            let parent_object = parent_item.object()?;
            if let Ok(tree) = parent_object.into_tree() {
                tree
            } else {
                return Err(GitError::NotFound);
            }
        };

        let potential_entry = tree
            .iter()
            .filter(|entry| entry.name_bytes() == &self.path[0][..])
            .next();
        if let Some(entry) = potential_entry {
            Ok(entry.to_object(&self.repo.repo)?)
        } else {
            Err(GitError::NotFound)
        }
    }

    pub fn content(&self) -> Result<Vec<u8>, GitError> {
        if let Ok(blob) = self.object()?.into_blob() {
            Ok(blob.content().to_vec())
        } else {
            Err(GitError::IsDir)
        }
    }

    // Method: canExist
    // Method: isDir, isFile (?)
    // Find out: Where to put actual file editing/committing
}

#[derive(Debug)]
pub enum GitError {
    GitError(git2::Error),
    NotFound,
    IsDir,
}
impl std::error::Error for GitError {}
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &GitError::GitError(ref err) => write!(f, "Git error: {}", err),
            &GitError::NotFound => write!(f, "Not found"),
            &GitError::IsDir => write!(f, "Is directory"),
        }
    }
}
impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::GitError(err)
    }
}
