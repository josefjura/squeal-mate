use std::fmt::Display;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) enum Entry {
    Directory(String),
    File(String, bool),
}

pub(crate) trait Name {
    fn get_name(self) -> String;
    fn get_name_ref(&self) -> &String;
}

impl Name for Entry {
    fn get_name(self) -> String {
        match self {
            Entry::File(name, _) => name,
            Entry::Directory(name) => name,
        }
    }
    fn get_name_ref(&self) -> &String {
        match self {
            Entry::File(name, _) => name,
            Entry::Directory(name) => name,
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name_ref().as_str())
    }
}

impl Entry {
    pub fn is_file(self) -> bool {
        if let Entry::File(_, _) = self {
            return true;
        } else {
            return false;
        }
    }
}
