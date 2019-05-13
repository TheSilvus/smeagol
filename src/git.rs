use std::io;
use std::io::Write;
use std::path::Path;

use git2::{
    Commit, ErrorCode, Object, ObjectType, Oid, Repository, RepositoryInitOptions, Signature,
    TreeBuilder,
};

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

    // TODO make path its own type
    pub fn parse_path(path: &str) -> Vec<Vec<u8>> {
        if path.len() == 0 {
            vec![]
        } else {
            path.split("/")
                .map(|s| s.bytes().collect::<Vec<_>>())
                .filter(|v| v.len() > 0)
                .collect::<Vec<_>>()
        }
    }
}

pub struct GitItem<'repo> {
    repo: &'repo GitRepository,
    path: Vec<Vec<u8>>,
}
impl<'repo> GitItem<'repo> {
    fn parent(&self) -> Result<GitItem<'repo>, GitError> {
        if self.path.len() == 0 {
            Err(GitError::NoParent)
        } else {
            let parent_path = self.path[..self.path.len() - 1].to_vec();
            Ok(GitItem {
                repo: self.repo,
                path: parent_path,
            })
        }
    }

    fn object(&self) -> Result<Object<'repo>, GitError> {
        // TODO cache object

        if self.path.len() == 0 {
            return Ok(self.repo.head()?.tree()?.into_object());
        }

        let parent_object = self.parent()?.object()?;
        let tree = if let Ok(tree) = parent_object.into_tree() {
            tree
        } else {
            return Err(GitError::NotFound);
        };

        let potential_entry = tree
            .iter()
            .filter(|entry| entry.name_bytes() == &self.path.last().unwrap()[..])
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

    pub fn exists(&self) -> Result<bool, GitError> {
        match self.object() {
            Ok(_) => Ok(true),
            Err(GitError::NotFound) => Ok(false),
            Err(err) => Err(err),
        }
    }
    pub fn could_exist(&self) -> Result<bool, GitError> {
        if self.path.len() == 0 || self.path.len() == 1 {
            Ok(true)
        } else {
            let parent = self.parent()?;
            if parent.exists()? && parent.is_dir()? {
                Ok(false)
            } else {
                Ok(parent.could_exist()?)
            }
        }
    }

    pub fn is_dir(&self) -> Result<bool, GitError> {
        Ok(self.object()?.kind() == Some(ObjectType::Tree))
    }
    pub fn is_file(&self) -> Result<bool, GitError> {
        Ok(self.object()?.kind() == Some(ObjectType::Blob))
    }

    pub fn edit(&self, content: &[u8], message: &str) -> Result<(), GitError> {
        // TODO I create quite a few objects here that are never used in case of an error. They
        // would be removed by a git gc. Should I attempt to remove them myself?
        // TODO get original file mode
        let mut blob_writer = self.repo.repo.blob_writer(None)?;
        blob_writer.write(content)?;
        let blob_oid = blob_writer.commit()?;

        let head = self.repo.head()?;
        let head_tree = head.tree()?;
        let mut tree_builder = self.repo.repo.treebuilder(Some(&head_tree))?;

        self.add_to_tree(&mut tree_builder, &self.path, blob_oid)?;

        let tree_oid = tree_builder.write()?;
        let new_tree = self.repo.repo.find_tree(tree_oid)?;

        let signature = Signature::now("smeagol", "smeagol@smeagol")?;

        self.repo.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &new_tree,
            &[&head],
        )?;

        Ok(())
    }

    fn add_to_tree(
        &self,
        tree: &mut TreeBuilder,
        path: &[Vec<u8>],
        object: Oid,
    ) -> Result<(), GitError> {
        assert!(path.len() > 0);

        if path.len() == 1 {
            if let Some(entry) = tree.get(&path[0])? {
                if entry.kind() != Some(ObjectType::Blob) {
                    return Err(GitError::IsDir);
                }
            }
            // TODO changeble filemode
            tree.insert(&path[0], object, 0o100644)?;
            Ok(())
        } else {
            let mut subtree_builder = if let Some(entry) = tree.get(&path[0])? {
                if let Some(subtree) = entry.to_object(&self.repo.repo)?.as_tree() {
                    self.repo.repo.treebuilder(Some(subtree))?
                } else {
                    return Err(GitError::CannotCreate);
                }
            } else {
                self.repo.repo.treebuilder(None)?
            };

            self.add_to_tree(&mut subtree_builder, &path[1..], object)?;

            let subtree_oid = subtree_builder.write()?;
            tree.insert(&path[0], subtree_oid, 0o040000)?;

            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum GitError {
    Git(git2::Error),
    IO(io::Error),
    NotFound,
    NoParent,
    IsDir,
    CannotCreate,
}
impl std::error::Error for GitError {}
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &GitError::Git(ref err) => write!(f, "Git error: {}", err),
            &GitError::IO(ref err) => write!(f, "IO error: {}", err),
            &GitError::NotFound => write!(f, "Not found"),
            &GitError::NoParent => write!(f, "No parent"),
            &GitError::IsDir => write!(f, "Is directory"),
            &GitError::CannotCreate => write!(f, "Cannot create file at that location"),
        }
    }
}
impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::Git(err)
    }
}
impl From<io::Error> for GitError {
    fn from(err: io::Error) -> Self {
        GitError::IO(err)
    }
}

#[cfg(test)]
mod tests {
    use crate::git::GitError;
    use crate::GitRepository;
    use tempdir::TempDir;

    #[test]
    fn automatic_repo_creation() {
        let tmp_dir = TempDir::new("smeagol").unwrap();
        let path = tmp_dir.path();

        let _repo = GitRepository::new(path).unwrap();

        assert!(path.exists());
        assert!(path.join("HEAD").exists());
        assert!(path.is_dir());
    }

    #[test]
    fn path_parsing() {
        assert_eq!(GitRepository::parse_path(""), Vec::<Vec<u8>>::new());
        assert_eq!(GitRepository::parse_path("/"), Vec::<Vec<u8>>::new());
        assert_eq!(GitRepository::parse_path("//"), Vec::<Vec<u8>>::new());

        assert_eq!(
            GitRepository::parse_path("index.md"),
            vec!["index.md".bytes().collect::<Vec<_>>()],
        );
        assert_eq!(
            GitRepository::parse_path("/index.md"),
            vec!["index.md".bytes().collect::<Vec<_>>()],
        );
        assert_eq!(
            GitRepository::parse_path("//index.md"),
            vec!["index.md".bytes().collect::<Vec<_>>()],
        );
        assert_eq!(
            GitRepository::parse_path("index.md/"),
            vec!["index.md".bytes().collect::<Vec<_>>()],
        );

        assert_eq!(
            GitRepository::parse_path("some/index.md"),
            vec![
                "some".bytes().collect::<Vec<_>>(),
                "index.md".bytes().collect::<Vec<_>>()
            ],
        );
        assert_eq!(
            GitRepository::parse_path("/some/index.md"),
            vec![
                "some".bytes().collect::<Vec<_>>(),
                "index.md".bytes().collect::<Vec<_>>()
            ],
        );
        assert_eq!(
            GitRepository::parse_path("some//index.md"),
            vec![
                "some".bytes().collect::<Vec<_>>(),
                "index.md".bytes().collect::<Vec<_>>()
            ],
        );
        assert_eq!(
            GitRepository::parse_path("some/index.md/"),
            vec![
                "some".bytes().collect::<Vec<_>>(),
                "index.md".bytes().collect::<Vec<_>>()
            ],
        );
    }

    #[test]
    fn root_always_exists() {
        let tmp = TempDir::new("smeagol").unwrap();
        let repo = GitRepository::new(tmp.path()).unwrap();

        let path = GitRepository::parse_path("");
        let item = repo.item(path).unwrap();

        assert!(item.exists().unwrap());
        assert!(item.is_dir().unwrap());
    }

    #[test]
    fn edit_file() {
        let tmp = TempDir::new("smeagol").unwrap();
        let repo = GitRepository::new(tmp.path()).unwrap();

        let path = GitRepository::parse_path("index.md");
        let item = repo.item(path).unwrap();

        assert!(!item.exists().unwrap());
        assert!(item.could_exist().unwrap());
        match item.content() {
            Err(GitError::NotFound) => {}
            _ => panic!(),
        }

        let file_content = "This is a file.".bytes().collect::<Vec<u8>>();
        item.edit(&file_content, "Commit message").unwrap();

        assert!(item.is_file().unwrap());
        assert!(!item.is_dir().unwrap());
        assert_eq!(item.content().unwrap(), file_content);

        let file_content = "This is an edited file.".bytes().collect::<Vec<u8>>();
        item.edit(&file_content, "Commit message 2").unwrap();

        assert_eq!(item.content().unwrap(), file_content);
    }

    #[test]
    fn edit_file_dir() {
        let tmp = TempDir::new("smeagol").unwrap();
        let repo = GitRepository::new(tmp.path()).unwrap();

        let path = GitRepository::parse_path("test/index.md");
        let item = repo.item(path).unwrap();
        let dir_item = item.parent().unwrap();

        assert!(!dir_item.exists().unwrap());
        assert!(!item.exists().unwrap());
        assert!(dir_item.could_exist().unwrap());
        assert!(item.could_exist().unwrap());
        match item.content() {
            Err(GitError::NotFound) => {}
            _ => panic!(),
        }

        let file_content = "This is a file.".bytes().collect::<Vec<u8>>();
        item.edit(&file_content, "Commit message").unwrap();

        assert!(dir_item.is_dir().unwrap());
        assert!(!dir_item.is_file().unwrap());
        assert!(item.is_file().unwrap());
        assert!(!item.is_dir().unwrap());
        assert_eq!(item.content().unwrap(), file_content);
    }

    #[test]
    fn could_exist() {
        let tmp = TempDir::new("smeagol").unwrap();
        let repo = GitRepository::new(tmp.path()).unwrap();

        let path = GitRepository::parse_path("test/index.md");
        let item = repo.item(path).unwrap();

        item.edit(&vec![], "commit").unwrap();

        let path2 = GitRepository::parse_path("test/index.md/something.md");
        let item2 = repo.item(path2).unwrap();
        assert!(!item2.could_exist().unwrap());

        match item2.content() {
            Err(GitError::NotFound) => {}
            _ => panic!(),
        }
        match item2.edit(&vec![], "commit") {
            Err(GitError::CannotCreate) => {}
            _ => panic!(),
        }
    }
}
