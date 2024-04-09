use std::{fmt::Display, path::PathBuf};

use crate::utils::{PathError, PathWrapper};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) enum Entry {
    Directory(String),
    File(PathWrapper),
}

pub(crate) trait Name {
    fn get_filename(self) -> String;
    fn get_filename_ref(&self) -> &String;
}

impl Name for Entry {
    fn get_filename(self) -> String {
        match self {
            Entry::File(name) => match name {
                PathWrapper::Filename(name) => name,
                PathWrapper::Relative {
                    relative_dir: _,
                    filename,
                } => filename,
                PathWrapper::Absolute {
                    absolute_dir: _,
                    filename,
                } => filename,
            },
            Entry::Directory(name) => name,
        }
    }

    fn get_filename_ref(&self) -> &String {
        match self {
            Entry::File(name) => match name {
                PathWrapper::Filename(name) => name,
                PathWrapper::Relative {
                    relative_dir: _,
                    filename,
                } => filename,
                PathWrapper::Absolute {
                    absolute_dir: _,
                    filename,
                } => filename,
            },
            Entry::Directory(name) => name,
        }
    }
}

impl Entry {
    pub fn get_full_path(&self) -> Result<PathBuf, PathError> {
        match self {
            Entry::Directory(path) => Ok(PathBuf::from(path)),
            Entry::File(wrapper) => Ok(wrapper.get_full_path()?),
        }
    }

    pub fn get_paths(&self) -> Result<Vec<Entry>, PathError> {
        match self {
            Entry::Directory(path) => Ok(files_from_directory(path.clone())?),
            Entry::File(_) => Ok(vec![self.clone()]),
        }
    }
}

fn files_from_directory(path: String) -> Result<Vec<Entry>, PathError> {
    let dir = std::fs::read_dir(path).map_err(|_| PathError::CantReadDirectoryContents)?;
    let mut files = vec![];
    for entry in dir {
        let entry = entry.map_err(|_| PathError::CantReadFile)?;
        let path = entry.path();

        let absolute_dir = path.parent().unwrap().to_path_buf();
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        let e = Entry::File(PathWrapper::Absolute {
            absolute_dir,
            filename,
        });

        let items = e.get_paths()?;
        files.extend(items);
    }
    Ok(files)
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_filename_ref().as_str())
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum ResultState {
    Finished,
    Running,
    Error,
    None,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct ResultLine {
    pub result: Entry,
    pub state: ResultState,
    pub error: Option<String>,
    pub elapsed: Option<u128>,
}

impl ResultLine {
    pub fn none(entry: &Entry) -> Self {
        Self {
            error: None,
            result: entry.clone(),
            state: ResultState::None,
            elapsed: None,
        }
    }
}
