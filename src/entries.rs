use std::fmt::Display;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) enum Entry {
    Directory(String),
    File(String),
}

pub(crate) trait Name {
    fn get_name(&self) -> &String;
}

impl Name for Entry {
    fn get_name(&self) -> &String {
        match self {
            Entry::File(name) => name,
            Entry::Directory(name) => name,
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}
