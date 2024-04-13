use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use color_eyre::eyre;
use walkdir::{DirEntry, WalkDir};

use crate::entries::ListEntry;

#[derive(Debug)]
pub enum RepositoryError {
    DoesNotExist,
    IOError(String),
    NotUTF8,
}

pub struct Repository {
    root: PathBuf,
    root_str: String,
    path: Vec<String>,
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
            Ok(Self {
                root,
                root_str,
                path: vec![],
            })
        } else {
            Err(RepositoryError::DoesNotExist)
        }
    }

    pub fn base_as_str(&self) -> String {
        self.root_str.clone()
    }

    pub fn base_as_path_buf(&self) -> PathBuf {
        self.root.clone()
    }

    pub fn current_as_path_buf(&self) -> PathBuf {
        self.path
            .iter()
            .fold(self.root.clone(), |acc, item| acc.join(item))
    }

    #[allow(unused)]
    pub fn current_relative_as_path_buf(&mut self) -> PathBuf {
        PathBuf::from(self.current_relative_as_str())
    }

    #[allow(unused)]
    pub fn current_relative_as_str(&self) -> String {
        let c = self.current_as_path_buf();
        let b = self.base_as_str();

        c.to_str().unwrap().replace(&b, "")
    }

    pub fn open_directory(&mut self, directory_name: &str) {
        self.path.push(directory_name.into());
    }

    pub fn leave_directory(&mut self) -> Option<String> {
        self.path.pop()
    }

    pub fn read_files_in_directory(&self) -> eyre::Result<Vec<String>> {
        let current = self.current_as_path_buf();
        let base = self.base_as_path_buf();
        let entries = read_dir(current)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let path_str = String::from(path.to_str().unwrap());
                let file_name = path.file_name()?.to_str()?;

                if file_name.starts_with('_') || file_name.starts_with('.') || path.is_dir() {
                    return None;
                }
                if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
                    let relative_path = path_str.replace(base.to_str().unwrap(), "");
                    let fixed = relative_path.trim_start_matches(std::path::MAIN_SEPARATOR);

                    Some(fixed.into())
                } else {
                    None
                }
            })
            .collect();

        Ok(entries)
    }

    pub fn get_children(&self, path: String) -> Vec<String> {
        let base = self.base_as_path_buf();
        let path = base.join(path);

        if !path.is_dir() {
            return vec![];
        }

        WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .filter(|f| f.path().extension().map(|p| p == "sql").unwrap_or(false))
            .map(|f| f.path().to_str().unwrap().to_string())
            .map(|f| {
                f.replace(base.to_str().unwrap(), "")
                    .trim_start_matches(std::path::MAIN_SEPARATOR)
                    .to_string()
            })
            .collect()
    }

    pub fn read_files_after(&self, from: &str) -> Vec<String> {
        let current = self.current_as_path_buf();
        let base = self.base_as_str().to_owned();
        let target = current.join(from);
        let target = target.to_str().unwrap_or_default();

        let files: Vec<String> = WalkDir::new(&base)
            //.sort_by_file_name()
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .skip_while(|f| f.path().to_str().unwrap() != target)
            .filter_map(|f| {
                let path = f.path();
                if path.extension()? == "sql" {
                    let relative_path = path.strip_prefix(&base).ok()?;
                    Some(
                        relative_path
                            .to_str()?
                            .trim_start_matches(std::path::MAIN_SEPARATOR)
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .collect();

        files
    }

    pub fn read_files_after_in_directory(&self, from: &str) -> eyre::Result<Vec<String>> {
        let current = self.current_as_path_buf();
        let base = self.base_as_path_buf();
        let entries = read_dir(current)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let path_str = String::from(path.to_str().unwrap());
                let file_name = path.file_name()?.to_str()?;

                if file_name.starts_with('_') || file_name.starts_with('.') || path.is_dir() {
                    return None;
                }

                if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
                    let relative_path = path_str.replace(base.to_str().unwrap(), "");
                    let fixed = relative_path
                        .trim_start_matches(std::path::MAIN_SEPARATOR)
                        .to_owned();
                    Some((fixed, file_name.to_owned()))
                } else {
                    None
                }
            })
            .skip_while(|path| path.1 != from)
            .map(|path| path.0)
            .collect();

        Ok(entries)
    }

    pub fn read_entries_in_current_directory(&self) -> Vec<ListEntry> {
        let current = self.current_as_path_buf();
        let base = self.base_as_path_buf();

        let mut entries = match read_dir(current) {
            Ok(entries) => entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    let path_str = String::from(path.to_str().unwrap());
                    let file_name = path.file_name()?.to_str()?;

                    if file_name.starts_with('_') || file_name.starts_with('.') {
                        return None;
                    }
                    let relative_path = path_str.replace(base.to_str().unwrap(), "");
                    let fixed = relative_path.trim_start_matches(std::path::MAIN_SEPARATOR);

                    // Check if it's a directory or a file with .sql extension
                    if path.is_dir() {
                        Some(ListEntry {
                            is_directory: true,
                            relative_path: fixed.into(),
                            name: file_name.into(),
                            selected: false,
                        })
                    } else if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
                        Some(ListEntry {
                            is_directory: false,
                            relative_path: fixed.into(),
                            name: file_name.into(),
                            selected: false,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            Err(e) => {
                eprintln!("Failed to read directory: {}", e);
                Vec::new()
            }
        };

        entries.sort();

        entries
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('_') || s.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    #[test]
    fn repository_path_success() {
        let path = ".tests/repository/success";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());
        assert_eq!(String::from(path), r.unwrap().base_as_str())
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

    #[test]
    fn repository_path_movement() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let mut repository = r.unwrap();
        assert_eq!(String::from(path), repository.base_as_str());

        let entries = repository.read_entries_in_current_directory();
        assert_eq!(3, entries.len());

        repository.open_directory("dir2");
        let entries = repository.read_entries_in_current_directory();
        assert_eq!(1, entries.len());

        repository.leave_directory();
        let entries = repository.read_entries_in_current_directory();
        assert_eq!(3, entries.len());
    }

    #[test]
    fn repository_path_relative() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let mut repository = r.unwrap();

        repository.open_directory("dir2");

        assert_eq!("/dir2", repository.current_relative_as_str())
    }

    #[test]
    fn repository_getchildren_positive() {
        let path = ".tests/repository";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir1".into());
        assert_eq!(6, children.len());
    }

    #[test]
    fn repository_getchildren_positive2() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir2".into());
        assert_eq!(1, children.len());
    }

    #[test]
    fn repository_getchildren_positive3() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir3".into());
        assert_eq!(4, children.len());
    }

    #[test]
    fn repository_select_all_after() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let mut repository = r.unwrap();

        repository.open_directory("dir3");

        let children = repository.read_files_after("file4.sql");
        assert_eq!(4, children.len());
    }

    #[test]
    fn repository_select_all_after2() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let mut repository = r.unwrap();

        repository.open_directory("dir2");

        let children = repository.read_files_after("file2.sql");
        assert_eq!(6, children.len());
    }

    #[test]
    fn repository_select_all_after3() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert_eq!(true, r.is_ok());

        let mut repository = r.unwrap();

        repository.open_directory("dir3");

        let children = repository.read_files_after("file6.sql");
        assert_eq!(2, children.len());
    }
}
