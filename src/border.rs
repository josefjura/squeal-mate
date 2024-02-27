use std::io::{self, Error};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Print, Stylize},
};

pub fn draw_rect(
    stdout: &mut io::Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Result<(), Error> {
    let row: u16 = y;
    let column: u16 = x;
    let tl = (column, row);
    let tr = (column + width - 1, row);
    let bl = (column, height - 1 + row);
    let br = (column + width - 1, height - 1 + row);

    // ┌─┐
    // │ │
    // └─┘

    let line = "─".repeat(width as usize - 2).yellow();

    queue!(
        stdout,
        MoveTo(tl.0, tl.1),
        Print("┌".yellow()),
        Print(&line),
        Print("┐".yellow())
    )?;
    queue!(
        stdout,
        MoveTo(bl.0, br.1),
        Print("└".yellow()),
        Print(&line),
        Print("┘".yellow())
    )?;

    for col in tl.1 + 1..bl.1 {
        queue!(stdout, MoveTo(tl.0, col), Print("│".yellow()))?;
        queue!(stdout, MoveTo(tr.0, col), Print("│".yellow()))?;
    }

    Ok(())
}

#[test]
fn simple_draw() {
    let mut io = io::stdout();

    let _ = draw_rect(&mut io, 0, 0, 5, 5);
    let _ = draw_rect(&mut io, 20, 0, 8, 8);
    let _ = draw_rect(&mut io, 5, 5, 5, 5);
}
