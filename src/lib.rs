mod generate_puz;
mod parse_grid;

use std::collections::HashMap;
use std::fmt;
use std::iter::zip;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
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

    pub fn is_wall(&self) -> bool {
        matches!(self, CrosswordCell::Wall)
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

impl Crossword {
    fn validate(&self) -> Result<(), JsErrors> {
        let mut issues = JsErrors::new();
        let (across, down) = self.expected_grid_nums();
        if let Err(err) = Self::validate_clues(&across, &self.across_clues) {
            issues.push("across_clues", err);
        }
        if let Err(err) = Self::validate_clues(&down, &self.down_clues) {
            issues.push("down_clues", err);
        }

        if issues.is_empty() { Ok(()) }
        else { Err(issues) }
    }

    fn validate_clues(expected: &[u16], actual: &[(u16, String)]) -> Result<(), String> {
        if actual.windows(2).any(|w| w[0] >= w[1]) {
            return Err("found misordered clues. Clue numbers must be strictly increasing".into());
        }
        if expected.len() != actual.len() {
            return Err(format!("expected {} clues, found {}", expected.len(), actual.len()));
        }
        let mismatch =
            zip(expected, actual)
            .map(|(a, (b, _))| (a, b))
            .filter(|(a, b)| a != b)
            .next();
        if let Some((exp, act)) = mismatch {
            return if exp < act { Err(format!("missing clue {exp}")) }
            else { Err(format!("found extraneous clue {act}")) };
        }
        Ok(())
    }

    fn expected_grid_nums(&self) -> (Vec<u16>, Vec<u16>) {
        let width = self.width as usize;
        let mut across = Vec::new();
        let mut down = Vec::new();
        let mut num = 1;
        for (idx, cell) in self.cells.iter().enumerate() {
            if cell.is_wall() { continue; }
            let x = idx % width;
            let y = idx / width;
            let is_across = x == 0 || self.cells[idx - 1].is_wall();
            let is_down = y == 0 || self.cells[idx - width].is_wall();
            if is_across {
                across.push(num);
            }
            if is_down {
                down.push(num);
            }
            if is_across || is_down {
                num += 1;
            }
        }
        (across, down)
    }
}

fn parse_clue_prefix(line: &str) -> Option<(u16, String)> {
    if let Some((num, clue)) = line.trim_start().split_once('.') {
        if let Ok(num) = num.parse() {
            return Some((num, clue.to_string()));
        }
    }
    None
}

fn parse_clue_block(clue_block: &str) -> Result<Vec<(u16, String)>, &'static str> {
    let mut clues = Vec::new();
    let mut lines = clue_block.lines();
    let first_line = lines.next()
        .ok_or("no clues provided")?;
    let (mut cur_num, mut cur_clue) = parse_clue_prefix(first_line)
        .ok_or("clues must include line numbers: ` 1. Clue`")?;
    for line in lines {
        if let Some((num, clue)) = parse_clue_prefix(line) {
            clues.push((cur_num, cur_clue));
            cur_num = num;
            cur_clue = clue;
        } else {
            cur_clue.push('\n');
            cur_clue.push_str(line);
        }
    }
    clues.push((cur_num, cur_clue));
    Ok(clues)
}

#[wasm_bindgen]
#[derive(Deserialize)]
pub struct CrosswordInput {
    image: Vec<u8>,
    across_clues: String,
    down_clues: String,
    title: String,
    author: String,
    copyright: String,
    notes: String,
}

#[wasm_bindgen]
impl CrosswordInput {
    #[wasm_bindgen(constructor)]
    pub fn new(blob: JsValue) -> CrosswordInput {
        serde_wasm_bindgen::from_value(blob).unwrap()
    }
}

#[derive(Default)]
pub struct JsErrors {
    errors: HashMap<String, String>,
}

impl JsErrors {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    fn push(&mut self, section: &str, msg: String) {
        self.errors.insert(section.into(), msg);
    }
}

impl Into<JsValue> for JsErrors {
    fn into(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.errors)
            .expect("map of strings to strings should be serializable")
    }
}

#[wasm_bindgen]
pub fn generate_puz_file(input: CrosswordInput) -> Result<Vec<u8>, JsErrors> {
    set_panic_hook();

    let mut errors = JsErrors::new();
    let CrosswordInput { image, across_clues, down_clues, title, author, copyright, notes } = input;
    let across_clues = parse_clue_block(&across_clues);
    if let Err(msg) = across_clues {
        errors.push("across_clues", msg.into());
    }
    let down_clues = parse_clue_block(&down_clues);
    if let Err(msg) = down_clues {
        errors.push("down_clues", msg.into());
    }
    let img = image::load_from_memory(&image);
    if let Err(ref msg) = img {
        errors.push("image", format!("could not load image: {msg}"));
    }
    let (Ok(across_clues), Ok(down_clues), Ok(img)) = (across_clues, down_clues, img)
        else { return Err(errors) };
    let CrosswordGrid { width, height, cells } = parse_grid::parse_crossword(img);
    let xword = Crossword {
        width, height, cells,
        title, author, copyright, notes,
        across_clues, down_clues,
    };
    xword.validate()?;
    Ok(xword.as_puz())
}

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
