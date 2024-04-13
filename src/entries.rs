use std::fmt::Display;

// #[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
// pub(crate) enum Entry {
//     Directory(String),
//     File(PathWrapper),
// }

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]

pub struct ListEntry {
    pub relative_path: String,
    pub name: String,
    pub selected: bool,
    pub is_directory: bool,
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

// #[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
// pub enum ResultState {
//     Finished,
//     Running,
//     Error,
//     None,
// }

// #[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
// pub struct ResultLine {
//     pub relative_path: String,
//     pub state: ResultState,
//     pub error: Option<String>,
//     pub elapsed: Option<u128>,
// }

// impl ResultLine {
//     pub fn none(entry: &str) -> Self {
//         Self {
//             error: None,
//             relative_path: entry.into(),
//             state: ResultState::None,
//             elapsed: None,
//         }
//     }
// }
