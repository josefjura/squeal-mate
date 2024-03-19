use std::{env, ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

#[derive(Debug)]
pub enum RepositoryError {
    DoesNotExist,
    IOError(String),
    NotUTF8,
}

pub struct Repository {
    root: PathBuf,
    root_str: String,
}

impl Repository {
    /// Attempts to store path, if it's valid and the file exists.
    /// Used for longer storage of paths.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Result<Repository, RepositoryError> = Repository::new("some/existing/file");
    /// assert_eq!(x.is_ok(), true);
    ///
    /// let x: Result<Repository, RepositoryError> = Repository::new("some/non-existing/file");
    /// assert_eq!(x.is_ok(), false);
    /// ```
    pub fn new(root: PathBuf) -> Result<Self, RepositoryError> {
        let root_str = root.to_str().ok_or(RepositoryError::NotUTF8)?.to_string();

        if root
            .try_exists()
            .map_err(|e| RepositoryError::IOError(e.to_string()))?
        {
            Ok(Self { root, root_str })
        } else {
            Err(RepositoryError::DoesNotExist)
        }
    }

    pub fn as_str(&self) -> String {
        self.root_str.clone()
    }

    pub fn as_path_buf(&self) -> PathBuf {
        self.root.clone()
    }
}

#[test]
fn repository_path_success() {
    let path = ".tests/repository/success";
    let r = Repository::new(PathBuf::from(path));

    assert_eq!(true, r.is_ok());
    assert_eq!(String::from(path), r.unwrap().as_str())
}

#[test]
fn repository_path_does_not_exist() {
    let r = Repository::new(PathBuf::from(".tests/repository/failure"));

    assert!(r.is_err());
    match r {
        Err(RepositoryError::DoesNotExist) => assert!(true),
        _ => assert!(false, "Expected RepositoryError::DoesNotExist"),
    }
}

#[test]
fn repository_path_is_not_utf8() {
    let non_utf8_bytes = vec![0xff, 0xff, 0xff];
    let non_utf8_os_string = OsString::from_vec(non_utf8_bytes);
    let non_utf8_path = PathBuf::from(non_utf8_os_string);

    let r = Repository::new(non_utf8_path);

    assert_eq!(true, r.is_err());
    match r {
        Err(RepositoryError::NotUTF8) => assert!(true),
        _ => assert!(false, "Expected RepositoryError::NotUTF8"),
    }
}
