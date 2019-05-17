use percent_encoding::define_encode_set;

use std::fmt;
use std::path::{Path as StdPath, PathBuf as StdPathBuf};

define_encode_set! {
    pub PERCENT_ENCODE_SET = [percent_encoding::DEFAULT_ENCODE_SET] | { '%' }
}

const PATH_SEPARATOR: u8 = '/' as u8;
const EXTENSION_SEPARATOR: u8 = '.' as u8;

// TODO I could add a separate referencetype for this structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    content: Vec<u8>,
}
impl Path {
    pub fn new() -> Path {
        Self::from_vec(vec![])
    }
    pub fn from_percent_encoded(s: &[u8]) -> Path {
        Self::from_vec(percent_encoding::percent_decode(s).collect::<Vec<_>>())
    }
    pub fn percent_encode(&self) -> String {
        percent_encoding::percent_encode(&self.content[..], PERCENT_ENCODE_SET).to_string()
    }

    fn from_vec(content: Vec<u8>) -> Path {
        let mut path = Path { content };
        path.normalize();
        path
    }

    fn normalize(&mut self) {
        self.content
            .dedup_by(|a, b| *a == PATH_SEPARATOR && *a == *b);
        // Remove leading and trailing slashes
        if self.content.first() == Some(&PATH_SEPARATOR) {
            self.content.remove(0);
        }
        if self.content.last() == Some(&PATH_SEPARATOR) {
            self.content.pop().unwrap();
        }
    }

    pub fn push<P: Into<Path>>(&mut self, path: P) {
        self.content.push(PATH_SEPARATOR);
        self.content.extend_from_slice(&path.into().content[..]);
        // Normalization is necessary because of some special cases
        self.normalize();
    }
    pub fn pop_first(&mut self) -> Option<Path> {
        let first_separator = self.content.iter().position(|b| *b == PATH_SEPARATOR);
        if let Some(index) = first_separator {
            let new_path = Path::from(self.content[..index].to_vec());
            self.content.drain(..index + 1);
            Some(new_path)
        } else if !self.is_empty() {
            let mut new_content = vec![];
            std::mem::swap(&mut new_content, &mut self.content);
            let new_path = Path::from(new_content);
            Some(new_path)
        } else {
            None
        }
    }

    fn last_separator(&self) -> Option<usize> {
        let mut index = None;
        for (i, b) in self.content.iter().enumerate().rev() {
            if *b == PATH_SEPARATOR {
                index = Some(i);
                break;
            }
        }
        index
    }

    pub fn filename(&self) -> Option<Path> {
        let index = self.last_separator();
        if let Some(index) = index {
            Some(Path::from_vec(self.content[index + 1..].to_vec()))
        } else if self.content.len() > 0 {
            Some(Path::from_vec(self.content[..].to_vec()))
        } else {
            None
        }
    }

    pub fn parent(&self) -> Option<Path> {
        let index = self.last_separator();

        if let Some(index) = index {
            Some(Path::from(self.content[0..index].to_vec()))
        } else if !self.is_empty() {
            Some(Path::new())
        } else {
            None
        }
    }

    // TODO rewrite this with slice return value
    pub fn extension(&self) -> Option<Vec<u8>> {
        if let Some(filename) = self.filename() {
            assert!(filename.bytes().len() > 0);

            for (i, c) in filename.bytes().iter().enumerate() {
                if i == 0 && *c == EXTENSION_SEPARATOR {
                    continue;
                } else if *c == EXTENSION_SEPARATOR {
                    return Some(filename.bytes()[i + 1..].to_vec());
                }
            }

            None
        } else {
            None
        }
    }

    pub fn segments<'a>(&'a self) -> impl Iterator<Item = &[u8]> + 'a {
        self.content.split(|b| *b == PATH_SEPARATOR)
    }

    pub fn is_empty(&self) -> bool {
        self.content.len() == 0
    }

    pub fn bytes(&self) -> &[u8] {
        &self.content[..]
    }
}

impl From<Vec<u8>> for Path {
    fn from(v: Vec<u8>) -> Path {
        Path::from_vec(v)
    }
}
impl From<String> for Path {
    fn from(s: String) -> Path {
        Path::from(s.into_bytes())
    }
}
impl From<&StdPath> for Path {
    fn from(p: &StdPath) -> Path {
        // Note: This conversion panics if the path is invalid unicode. It should therefore not be
        // used on untrusted data.
        // TODO implement this using TryFrom
        Path::from(p.to_str().unwrap().to_string())
    }
}
impl From<&Path> for StdPathBuf {
    fn from(p: &Path) -> StdPathBuf {
        // Note: This conversion panics if the path is invalid unicode. It should therefore not be
        // used on untrusted data.
        // TODO implement this using TryFrom
        StdPathBuf::from(String::from_utf8(p.content.clone()).unwrap())
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.content[..]))
    }
}

pub struct PathStringBuilder {
    path: Path,

    root: bool,
    dir: bool,
}
impl PathStringBuilder {
    pub fn new(path: Path) -> PathStringBuilder {
        PathStringBuilder {
            path,
            root: false,
            dir: false,
        }
    }

    pub fn root(&mut self, root: bool) -> &mut PathStringBuilder {
        self.root = root;
        self
    }
    pub fn dir(&mut self, dir: bool) -> &mut PathStringBuilder {
        self.dir = dir;
        self
    }

    pub fn build_lossy(&self) -> String {
        let mut s = String::new();
        if self.root {
            s.push('/');
        }
        s += &self.path.to_string();
        if self.dir {
            s.push('/');
        }

        s
    }
    pub fn build_percent_encode(&self) -> String {
        let mut s = String::new();
        if self.root {
            s.push('/');
        }
        s += &self.path.percent_encode();
        if self.dir {
            s.push('/');
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use crate::Path;
    use std::path::PathBuf as StdPathBuf;

    #[test]
    fn from_vec() {
        let vec = vec![1, 2, 3, 4, 5, 6];
        let path = Path::from(vec.clone());
        assert_eq!(path.content, vec);
    }
    #[test]
    fn from_string() {
        let s = "path/test";
        let path = Path::from(s.to_string());
        assert_eq!(path.content, s.bytes().collect::<Vec<_>>());
    }
    #[test]
    fn from_std_path() {
        let path_string = "path/test";
        let std_path = StdPathBuf::from(path_string);
        let path = Path::from(std_path.as_path());

        assert_eq!(path.content, path_string.bytes().collect::<Vec<_>>());
    }
    #[test]
    fn empty_path() {
        let path = Path::new();
        assert_eq!(path.content, Vec::<u8>::new());
    }

    #[test]
    fn normalize() {
        let path = Path::from("/abc//def/".to_string());
        assert_eq!(path.to_string(), "abc/def");
    }

    #[test]
    fn push() {
        let mut path1 = Path::new();
        let path2 = Path::from("abc".to_string());
        path1.push(path2);
        assert_eq!(path1, Path::from("abc".to_string()));

        let mut path1 = Path::from("abc".to_string());
        let path2 = Path::from("def".to_string());
        path1.push(path2);
        assert_eq!(path1.to_string(), "abc/def");
    }

    #[test]
    fn pop_first() {
        let mut path = Path::from("abc/def".to_string());
        let first = path.pop_first();
        assert_eq!(first, Some(Path::from("abc".to_string())));
        assert_eq!(path, Path::from("def".to_string()));

        let mut path = Path::from("abc".to_string());
        let first = path.pop_first();
        assert_eq!(first, Some(Path::from("abc".to_string())));
        assert_eq!(path, Path::new());

        let mut path = Path::new();
        let first = path.pop_first();
        assert_eq!(first, None);
        assert_eq!(path, Path::new());
    }

    #[test]
    fn parent() {
        let path = Path::from("".to_string());
        assert_eq!(path.parent(), None);

        let path = Path::from("abc".to_string());
        assert_eq!(path.parent(), Some(Path::new()));

        let path = Path::from("abc/def".to_string());
        assert_eq!(path.parent(), Some(Path::from("abc".to_string())));
    }

    #[test]
    fn filename() {
        let path = Path::new();
        assert_eq!(path.filename(), None);

        let path = Path::from("abc".to_string());
        assert_eq!(path.filename(), Some(Path::from("abc".to_string())));

        let path = Path::from("abc/def".to_string());
        assert_eq!(path.filename(), Some(Path::from("def".to_string())));
    }

    #[test]
    fn extension() {
        let path = Path::from("".to_string());
        assert_eq!(path.extension(), None);

        let path = Path::from("abc".to_string());
        assert_eq!(path.extension(), None);

        let path = Path::from(".abc".to_string());
        assert_eq!(path.extension(), None);

        let path = Path::from("a.abc".to_string());
        assert_eq!(path.extension(), Some("abc".to_string().into_bytes()));

        let path = Path::from(".a.abc".to_string());
        assert_eq!(path.extension(), Some("abc".to_string().into_bytes()));

        let path = Path::from("a.abc.def".to_string());
        assert_eq!(path.extension(), Some("abc.def".to_string().into_bytes()));
    }
}
