use std::path::PathBuf;

#[derive(Debug)]
pub enum RepositoryError {
    DoesNotExist,
    IOError(String),
    NotUTF8,
}

/// Holds the validated root directory of the migration repository.
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// Stores `root` if it is valid UTF-8 and exists.
    pub fn new(root: PathBuf) -> Result<Self, RepositoryError> {
        root.to_str().ok_or(RepositoryError::NotUTF8)?;

        if root
            .try_exists()
            .map_err(|e| RepositoryError::IOError(e.to_string()))?
        {
            Ok(Self { root })
        } else {
            Err(RepositoryError::DoesNotExist)
        }
    }

    pub fn base_as_path_buf(&self) -> PathBuf {
        self.root.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::OsString;

    #[cfg(unix)]
    fn non_utf8_os_string() -> OsString {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff, 0xff, 0xff])
    }

    #[cfg(windows)]
    fn non_utf8_os_string() -> OsString {
        use std::os::windows::ffi::OsStringExt;
        // An unpaired surrogate is not representable in UTF-8.
        OsString::from_wide(&[0xD800])
    }

    #[test]
    fn repository_path_success() {
        let path = ".tests/repository/success";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());
        assert_eq!(path, r.unwrap().base_as_path_buf().to_str().unwrap())
    }

    #[test]
    fn repository_path_does_not_exist() {
        let r = Repository::new(PathBuf::from(".tests/repository/failure"));

        assert!(r.is_err());
        assert!(matches!(r, Err(RepositoryError::DoesNotExist)));
    }

    #[test]
    fn repository_path_is_not_utf8() {
        let non_utf8_path = PathBuf::from(non_utf8_os_string());

        let r = Repository::new(non_utf8_path);

        assert!(r.is_err());
        assert!(matches!(r, Err(RepositoryError::NotUTF8)));
    }
}
