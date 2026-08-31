use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use color_eyre::eyre;
use walkdir::{DirEntry, WalkDir};

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

    pub fn read_files_in_directory(&self) -> eyre::Result<Vec<String>> {
        let entries = self
            .list_sql_file_entries(&self.current_as_path_buf())?
            .into_iter()
            .map(|(relative_path, _file_name)| relative_path)
            .collect();

        Ok(entries)
    }

    /// List `.sql` files directly inside `dir` (non-recursive), skipping hidden
    /// (`.`/`_`-prefixed) entries and subdirectories. `dir` should be relative
    /// to the repository root (matching `get_children`'s convention) - it is
    /// joined onto the root before reading. Results are relative to the root.
    pub fn list_sql_files_in_directory(&self, dir: &Path) -> eyre::Result<Vec<String>> {
        let target = self.base_as_path_buf().join(dir);
        let entries = self
            .list_sql_file_entries(&target)?
            .into_iter()
            .map(|(relative_path, _file_name)| relative_path)
            .collect();

        Ok(entries)
    }

    /// Shared "what counts as a listable SQL script" rule: non-recursive,
    /// skips hidden (`.`/`_`-prefixed) entries and subdirectories, requires a
    /// `.sql` extension. Returns each match's path (relative to the
    /// repository root) paired with its file name, in directory-entry order.
    fn list_sql_file_entries(&self, dir: &Path) -> eyre::Result<Vec<(String, String)>> {
        let base = self.base_as_path_buf();
        let entries = read_dir(dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let path_str = String::from(path.to_str().unwrap());
                let file_name = path.file_name()?.to_str()?.to_string();

                if file_name.starts_with('_') || file_name.starts_with('.') || path.is_dir() {
                    return None;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
                    return None;
                }

                let relative_path = path_str.replace(base.to_str().unwrap(), "");
                let fixed = relative_path
                    .trim_start_matches(std::path::MAIN_SEPARATOR)
                    .to_string();

                Some((fixed, file_name))
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
        let base = self.base_as_path_buf();
        let target = current.join(from);
        let target = target.as_path();

        WalkDir::new(&base)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .skip_while(|f| f.path() != target)
            .filter(|f| {
                f.file_type().is_file() && f.path().extension().unwrap_or_default() == "sql"
            })
            .filter_map(|f| {
                let path = f.path();
                let relative_path = path.strip_prefix(&base).ok()?;
                relative_path.to_str().map(|f| f.to_string())
            })
            .collect()
    }

    pub fn read_files_after_in_directory(&self, from: &str) -> eyre::Result<Vec<String>> {
        let current = self.current_as_path_buf();
        let entries = self
            .list_sql_file_entries(&current)?
            .into_iter()
            .skip_while(|(_relative_path, file_name)| file_name != from)
            .map(|(relative_path, _file_name)| relative_path)
            .collect();

        Ok(entries)
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
    use std::ffi::OsString;
    use tempfile::TempDir;

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
        assert_eq!(String::from(path), r.unwrap().base_as_str())
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

    #[test]
    fn repository_getchildren_positive() {
        let path = ".tests/repository";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir1".into());
        assert_eq!(6, children.len());
    }

    #[test]
    fn repository_getchildren_positive2() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir2".into());
        assert_eq!(1, children.len());
    }

    #[test]
    fn repository_getchildren_positive3() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.get_children("dir3".into());
        assert_eq!(4, children.len());
    }

    #[test]
    fn repository_select_all_after() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.read_files_after("dir3/file4.sql");
        assert_eq!(4, children.len());
    }

    #[test]
    fn repository_select_all_after2() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.read_files_after("dir2/file2.sql");
        assert_eq!(6, children.len());
    }

    #[test]
    fn repository_list_sql_files_in_directory_matches_read_files_in_directory() {
        let path = ".tests/repository/dir1";
        let repository = Repository::new(PathBuf::from(path)).unwrap();

        let mut via_current = repository.read_files_in_directory().unwrap();
        let mut via_arbitrary_dir = repository
            .list_sql_files_in_directory(Path::new(""))
            .unwrap();

        via_current.sort();
        via_arbitrary_dir.sort();

        assert_eq!(via_current, via_arbitrary_dir);
        assert_eq!(via_current, vec!["file1.sql".to_string()]);
    }

    #[test]
    fn repository_list_sql_files_in_directory_skips_hidden_and_non_sql() {
        let path = ".tests/repository/dir1";
        let repository = Repository::new(PathBuf::from(path)).unwrap();

        let mut files = repository
            .list_sql_files_in_directory(Path::new("dir3"))
            .unwrap();
        files.sort();

        // dir3 has file3.sql..file6.sql plus a file.notsql - only .sql files should show up
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            files,
            vec![
                format!("dir3{sep}file3.sql"),
                format!("dir3{sep}file4.sql"),
                format!("dir3{sep}file5.sql"),
                format!("dir3{sep}file6.sql"),
            ]
        );
    }

    #[test]
    fn repository_list_sql_files_in_directory_propagates_read_dir_errors() {
        let path = ".tests/repository/dir1";
        let repository = Repository::new(PathBuf::from(path)).unwrap();

        let result = repository.list_sql_files_in_directory(Path::new("does-not-exist"));

        assert!(result.is_err());
    }

    // `read_files_after_in_directory` walks `read_dir`'s raw (unspecified,
    // platform-dependent) entry order, so a directory with several files can't
    // assert a stable "skips these, keeps those" list across OSes. A
    // single-entry directory sidesteps the ordering question entirely while
    // still locking down that the matching file itself is kept, not skipped.
    #[test]
    fn repository_read_files_after_in_directory_keeps_the_matching_file() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("target.sql"), "SELECT 1;").unwrap();
        let repository = Repository::new(temp_dir.path().to_path_buf()).unwrap();

        let files = repository
            .read_files_after_in_directory("target.sql")
            .unwrap();

        assert_eq!(files, vec!["target.sql".to_string()]);
    }

    #[test]
    fn repository_read_files_after_in_directory_skips_hidden_and_non_sql() {
        let path = ".tests/repository/dir1";
        let repository = Repository::new(PathBuf::from(path)).unwrap();

        // "from" not found among dir1's direct .sql files (file1.sql only) -> empty
        let files = repository
            .read_files_after_in_directory("nonexistent.sql")
            .unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn repository_select_all_after3() {
        let path = ".tests/repository/dir1";
        let r = Repository::new(PathBuf::from(path));

        assert!(r.is_ok());

        let repository = r.unwrap();

        let children = repository.read_files_after("dir3/file6.sql");
        assert_eq!(2, children.len());
    }
}
