use std::fmt::Display;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]

pub struct ListEntry {
    pub relative_path: String,
    pub name: String,
    pub selected: bool,
    pub is_directory: bool,
    pub status: EntryStatus,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
pub enum EntryStatus {
    NeverStarted,
    Finished(bool),
    Changed,
    Unknown,
    Directory,
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}
