use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};

use crate::utils::round_up_division;

#[derive(Debug, PartialEq)]
pub(crate) struct List {
    pub entries: Vec<String>,
    pub cursor: usize,
    pub page_index: usize,
    pub height: usize,
}

impl List {
    pub(crate) fn move_cursor_up(&mut self) {
        let page_size = self.get_page_entries().len();
        let new_cursor = if self.cursor == 0 {
            page_size - 1
        } else {
            self.cursor - 1
        };

        self.cursor = new_cursor;
    }

    pub(crate) fn move_cursor_down(&mut self) {
        let page_size = self.get_page_entries().len();
        let new_cursor = if self.cursor == page_size - 1 {
            0
        } else {
            self.cursor + 1
        };

        self.cursor = new_cursor;
    }

    pub(crate) fn move_page_forward(&mut self) {
        let page_count = self.get_page_count();
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

    fn get_page_entries(&self) -> Vec<&String> {
        self.entries
            .iter()
            .skip(self.height * self.page_index)
            .take(self.height)
            .collect::<Vec<&String>>()
    }

    pub(crate) fn draw(&self, stdout: &mut std::io::Stdout) -> Result<(), std::io::Error> {
        let page = self.get_page_entries();

        for line in 0..self.height {
            if let Some(item) = page.get(line) {
                if line == self.cursor {
                    queue!(
                        stdout,
                        MoveTo(0, line as u16),
                        Print(format!(" > {item}").blue()),
                        Clear(ClearType::UntilNewLine)
                    )?;
                } else {
                    queue!(
                        stdout,
                        MoveTo(0, line as u16),
                        Print(format!("   {item}").white()),
                        Clear(ClearType::UntilNewLine)
                    )?;
                }
            } else {
                queue!(
                    stdout,
                    MoveTo(0, line as u16),
                    Clear(ClearType::CurrentLine)
                )?;
            }
        }

        Ok(())
    }

    pub(crate) fn resize(&mut self, height: usize) {
        self.cursor = 0;
        self.page_index = 0;
        self.height = height;
    }

    pub(crate) fn get_selection(&self) -> &str {
        let page = self.get_page_entries();

        page.get(self.cursor).unwrap()
    }

    pub(crate) fn get_page_count(&self) -> usize {
        round_up_division(self.entries.len(), self.height)
    }
}

#[test]
fn lets_see() {
    //let mut stdout = std::io::stdout();

    let mut list = List {
        cursor: 0,
        page_index: 0,
        height: 3,
        entries: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            .iter()
            .map(|f| f.to_string())
            .collect(),
    };

    assert_eq!("0", list.get_selection());
    list.move_cursor_down();
    assert_eq!("1", list.get_selection());
    list.move_cursor_down();
    assert_eq!("2", list.get_selection());

    list.move_page_back();
    assert_eq!("10", list.get_selection());

    list.move_page_forward();
    list.move_cursor_down();
    list.move_page_back();

    assert_eq!("10", list.get_selection());

    list.move_cursor_down();
    assert_eq!("9", list.get_selection());

    list.move_page_forward();
    assert_eq!("0", list.get_selection());
    assert_eq!(4, list.get_page_count());

    list.resize(5);
    assert_eq!(3, list.get_page_count());
    list.resize(11);
    assert_eq!(1, list.get_page_count());
}
