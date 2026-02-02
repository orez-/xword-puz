mod generate_puz;
mod generate_ipuz;
mod multi_error;
mod serde_lit;
mod validation;

use crate::validation::ClueError;
use serde::{Deserialize, Deserializer, Serialize};
use wasm_bindgen::prelude::*;

pub type MultiError = crate::multi_error::MultiError<ValidationError>;

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("expected {expected} clues, found {actual}")]
    MismatchedClueCount { expected: usize, actual: usize },
    #[error("found misordered clues. Clue numbers must be strictly increasing")]
    MisorderedClues,
    #[error("missing clue #{0}")]
    MissingClue(u16),
    #[error("found extraneous clue #{0}")]
    ExtraClue(u16),
    #[error("hard limit of 100 unique rebuses (found {0})")]
    TooManyRebuses(usize),
    #[error("expected {} grid elements ({width}x{height}), but found {grid_len}", width * height)]
    InvalidGridSize {
        width: u8,
        height: u8,
        grid_len: usize,
    },
}

impl From<ClueError> for ValidationError {
    fn from(err: ClueError) -> ValidationError {
        match err {
            ClueError::MismatchedClueCount { expected, actual } =>
                ValidationError::MismatchedClueCount { expected, actual },
            ClueError::MisorderedClues => ValidationError::MisorderedClues,
            ClueError::MissingClue(clue) => ValidationError::MissingClue(clue),
            ClueError::ExtraClue(clue) => ValidationError::ExtraClue(clue),
        }
    }
}

impl Serialize for ValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Debug)]
pub enum CrosswordCell {
    Empty,
    Char(char),
    Rebus(String),
    Wall,
}

impl CrosswordCell {
    fn is_wall(&self) -> bool {
        matches!(self, CrosswordCell::Wall)
    }
}

impl<'de> Deserialize<'de> for CrosswordCell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let maybe_s: Option<String> = Deserialize::deserialize(deserializer)?;
        Ok(match maybe_s {
            Some(s) => {
                let mut chrs = s.chars();
                match (chrs.next(), chrs.next()) {
                    (None, _) => CrosswordCell::Empty,
                    (_, Some(_)) => CrosswordCell::Rebus(s),
                    (Some(c), None) => CrosswordCell::Char(c),
                }
            }
            None => CrosswordCell::Wall,
        })
    }
}

/// Validated crossword struct
pub struct Crossword {
    width: u8,
    height: u8,
    grid: Vec<CrosswordCell>,
    across_clues: Vec<(u16, String)>,
    down_clues: Vec<(u16, String)>,
    title: String,
    author: String,
    copyright: String,
    notes: String,
}

impl Crossword {
    fn grid(&self) -> Grid<'_> {
        Grid {
            width: self.width,
            height: self.height,
            grid: &self.grid,
        }
    }
}

#[wasm_bindgen]
pub fn generate_puz(blob: JsValue) -> Result<Vec<u8>, MultiError> {
    let xword: CrosswordArgs =
        serde_wasm_bindgen::from_value(blob).expect("js object should be well-formed");
    let xword = xword.validate()?;
    Ok(xword.to_puz())
}

// ===

/// Simple data struct for the crossword object.
/// Can be converted into a `Crossword`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosswordArgs {
    pub width: u8,
    pub height: u8,
    pub grid: Vec<CrosswordCell>,
    pub across_clues: Vec<(u16, String)>,
    pub down_clues: Vec<(u16, String)>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub copyright: String,
    #[serde(default)]
    pub notes: String,
}

impl CrosswordArgs {
    pub fn validate(self) -> Result<Crossword, MultiError> {
        let mut issues = MultiError::new();

        let expected_len = self.width as usize * self.height as usize;
        if self.grid.len() != expected_len {
            let err = ValidationError::InvalidGridSize {
                width: self.width,
                height: self.height,
                grid_len: self.grid.len(),
            };
            issues.insert("grid", err);
            // catastrophic issue: return early.
            return Err(issues);
        }

        if let Err(err) = self.validate_rebuses() {
            issues.insert("grid", err);
        }

        let (across, down) = self.grid().expected_grid_nums();
        if let Err(err) = validation::validate_clues(&across, &self.across_clues) {
            issues.insert("across_clues", err.into());
        }
        if let Err(err) = validation::validate_clues(&down, &self.down_clues) {
            issues.insert("down_clues", err.into());
        }

        if !issues.is_empty() {
            return Err(issues);
        }

        let CrosswordArgs {
            width,
            height,
            grid,
            across_clues,
            down_clues,
            title,
            author,
            copyright,
            notes,
        } = self;
        let xword = Crossword {
            width,
            height,
            grid,
            across_clues,
            down_clues,
            title,
            author,
            copyright,
            notes,
        };
        Ok(xword)
    }

    fn validate_rebuses(&self) -> Result<(), ValidationError> {
        let mut seen_rebus = std::collections::HashSet::new();

        for cell in &self.grid {
            if let CrosswordCell::Rebus(s) = cell {
                seen_rebus.insert(s);
            }
        }

        let rebus_count = seen_rebus.len();
        if rebus_count >= 100 {
            return Err(ValidationError::TooManyRebuses(rebus_count));
        }
        Ok(())
    }

    fn grid(&self) -> Grid<'_> {
        Grid {
            width: self.width,
            height: self.height,
            grid: &self.grid,
        }
    }
}

#[derive(Debug)]
enum NumberedCell {
    Wall,
    Empty,
    Numbered {
        number: u16,
        is_across: bool,
        is_down: bool,
    }
}

struct Grid<'xword> {
    width: u8,
    height: u8,
    grid: &'xword [CrosswordCell],
}

impl<'xword> Grid<'xword> {
    fn iter_numbered(&self) -> impl Iterator<Item = NumberedCell> {
        let width = self.width as usize;
        let height = self.height as usize;
        let mut number = 1;
        self.grid.iter().enumerate().map(move |(idx, cell)| {
            if cell.is_wall() {
                return NumberedCell::Wall;
            }
            let x = idx % width;
            let y = idx / width;
            let left_wall = x == 0 || self.grid[idx - 1].is_wall();
            let right_wall = x + 1 == width || self.grid[idx + 1].is_wall();
            let up_wall = y == 0 || self.grid[idx - width].is_wall();
            let down_wall = y + 1 == height || self.grid[idx + width].is_wall();
            // one-long areas do NOT get clues.
            let is_across = left_wall && !right_wall;
            let is_down = up_wall && !down_wall;
            if !is_across && !is_down {
                return NumberedCell::Empty;
            }
            let out = NumberedCell::Numbered {
                number,
                is_across,
                is_down,
            };
            number += 1;
            out
        })
    }

    /// Given the shape of the grid, these are the numbers of each clue.
    fn expected_grid_nums(&self) -> (Vec<u16>, Vec<u16>) {
        let mut across = Vec::new();
        let mut down = Vec::new();
        for cell in self.iter_numbered() {
            if let NumberedCell::Numbered { number, is_across, is_down } = cell {
                if is_across {
                    across.push(number);
                }
                if is_down {
                    down.push(number);
                }
            }
        }
        (across, down)
    }
}
