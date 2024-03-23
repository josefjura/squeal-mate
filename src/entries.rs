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
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_filename_ref().as_str())
    }
}
