mod generate_puz;

enum CrosswordCell {
    Char(char),
    Rebus(String),
    Wall,
}

struct Crossword {
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

fn main() {
    println!("Hello, world!");
}
