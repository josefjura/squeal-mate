use crate::utils::round_up_division;

#[derive(Debug, PartialEq)]
pub(crate) struct FileList {
    pub entries: Vec<Entry>,
    pub cursor: usize,
    pub page_index: usize,
    pub height: usize,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) enum Entry {
    Directory(String),
    File(String),
}

trait Name {
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

impl FileList {
    pub(crate) fn move_cursor_up(&mut self) {
        let page_size = self.get_page_entries().len();

        if page_size == 0 {
            return;
        }

        let new_cursor = if self.cursor == 0 {
            page_size - 1
        } else {
            self.cursor - 1
        };

        self.cursor = new_cursor;
    }

    pub(crate) fn move_cursor_down(&mut self) {
        let page_size = self.get_page_entries().len();

        if page_size == 0 {
            return;
        }

        let new_cursor = if self.cursor == page_size - 1 {
            0
        } else {
            self.cursor + 1
        };

        self.cursor = new_cursor;
    }

    pub(crate) fn move_page_forward(&mut self) {
        let page_count = self.get_page_count();

        if page_count == 0 {
            return;
        }

        let new_page = if self.page_index == page_count - 1 {
            0
        } else {
            self.page_index + 1
        };

        self.page_index = new_page;

        let page_size = self.get_page_entries().len();

        if self.cursor > page_size - 1 {
            self.cursor = page_size - 1
        }
    }

    pub(crate) fn move_page_back(&mut self) {
        let page_count = self.get_page_count();

        if page_count == 0 {
            return;
        }

        let new_page = if self.page_index == 0 {
            page_count - 1
        } else {
            self.page_index - 1
        };

        self.page_index = new_page;

        let page_size = self.get_page_entries().len();
        if self.cursor > page_size - 1 {
            self.cursor = page_size - 1
        }
    }

    pub(crate) fn get_page_entries(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .skip(self.height * self.page_index)
            .take(self.height)
            .collect::<Vec<&Entry>>()
    }

    pub(crate) fn resize(&mut self, height: usize) {
        self.cursor = 0;
        self.page_index = 0;
        self.height = height;
    }

    pub(crate) fn get_selection(&self) -> Option<&Entry> {
        let page = self.get_page_entries();
        page.get(self.cursor).cloned()
    }

    pub(crate) fn set_entries(&mut self, new_entries: Vec<Entry>) {
        self.cursor = 0;
        self.page_index = 0;
        self.entries = new_entries;
    }

    pub(crate) fn get_page_count(&self) -> usize {
        round_up_division(self.entries.len(), self.height)
    }
}

#[test]
fn empty() {
    let mut list = FileList {
        cursor: 0,
        page_index: 0,
        height: 3,
        entries: vec![],
    };

    list.move_cursor_down();
    list.move_cursor_up();
    list.move_page_forward();
    list.move_page_back();
    assert_eq!(None, list.get_selection());
}

#[test]
fn positive() {
    //let mut stdout = std::io::stdout();

    let mut list = FileList {
        cursor: 0,
        page_index: 0,
        height: 3,
        entries: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            .iter()
            .map(|f| Entry::Directory(f.to_string()))
            .collect(),
    };

    assert_eq!("0", list.get_selection().unwrap().get_name());
    list.move_cursor_down();
    assert_eq!("1", list.get_selection().unwrap().get_name());
    list.move_cursor_down();
    assert_eq!("2", list.get_selection().unwrap().get_name());

    list.move_page_back();
    assert_eq!("10", list.get_selection().unwrap().get_name());

    list.move_page_forward();
    list.move_cursor_down();
    list.move_page_back();

    assert_eq!("10", list.get_selection().unwrap().get_name());

    list.move_cursor_down();
    assert_eq!("9", list.get_selection().unwrap().get_name());

    list.move_page_forward();
    assert_eq!("0", list.get_selection().unwrap().get_name());
    assert_eq!(4, list.get_page_count());

    list.resize(5);
    assert_eq!(3, list.get_page_count());
    list.resize(11);
    assert_eq!(1, list.get_page_count());
}
