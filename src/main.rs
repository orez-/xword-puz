mod generate_puz;
mod parse_grid;

use std::fmt;
use crate::parse_grid::CrosswordGrid;

#[derive(Debug)]
pub enum CrosswordCell {
    Char(char),
    Rebus(String),
    Wall,
}

impl CrosswordCell {
    pub fn empty() -> Self {
        Self::Char('A')
    }
}

pub struct Crossword {
    width: u8,
    height: u8,
    cells: Vec<CrosswordCell>,
    across_clues: Vec<(u16, String)>,
    down_clues: Vec<(u16, String)>,
    title: String,
    author: String,
    copyright: String,
    notes: String,
}

impl fmt::Debug for Crossword {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    title: {}", self.title)?;
        writeln!(f, "   author: {}", self.author)?;
        writeln!(f, "copyright: {}", self.copyright)?;
        writeln!(f, "    notes: {}", self.notes)?;
        let mut it = self.cells.iter();
        for _ in 0..self.height {
            for _ in 0..self.width {
                match it.next().unwrap() {
                    CrosswordCell::Char(c) => write!(f, "{}", c)?,
                    CrosswordCell::Rebus(_) => todo!(),
                    CrosswordCell::Wall => write!(f, "░")?,
                }
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

fn main() {
    println!("Hello, world!");
}
