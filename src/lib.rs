mod generate_puz;
mod multi_error;

use std::iter::zip;
use serde::{Deserialize, Deserializer, Serialize};
use wasm_bindgen::prelude::*;

pub type MultiError = crate::multi_error::MultiError<ValidationError>;

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("expected {expected} clues, found {actual}")]
    MismatchedClueCount {
        expected: usize,
        actual: usize,
    },
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
    }
}

impl Serialize for ValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
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
    where D: Deserializer<'de> {
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

#[wasm_bindgen]
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

#[wasm_bindgen]
pub fn generate_puz(blob: JsValue) -> Result<Vec<u8>, MultiError> {
    let xword: CrosswordArgs = serde_wasm_bindgen::from_value(blob)
        .expect("js object should be well-formed");
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
    fn validate(self) -> Result<Crossword, MultiError> {
        let mut issues = MultiError::new();

        let expected_len = self.width as usize * self.height as usize;
        if self.grid.len() != expected_len {
            let err = ValidationError::InvalidGridSize {
                width: self.width,
                height: self.height,
                grid_len: self.grid.len(),
            };
            issues.insert("grid_size", err);
            // catastrophic issue: return early.
            return Err(issues);
        }

        let (across, down) = self.expected_grid_nums();
        if let Err(err) = Self::validate_clues(&across, &self.across_clues) {
            issues.insert("across_clues", err);
        }
        if let Err(err) = Self::validate_clues(&down, &self.down_clues) {
            issues.insert("down_clues", err);
        }

        if !issues.is_empty() { return Err(issues) }

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

    fn validate_clues(expected: &[u16], actual: &[(u16, String)]) -> Result<(), ValidationError> {
        if actual.windows(2).any(|w| w[0] >= w[1]) {
            return Err(ValidationError::MisorderedClues);
        }
        if expected.len() != actual.len() {
            let expected = expected.len();
            let actual = actual.len();
            return Err(ValidationError::MismatchedClueCount { expected, actual });
        }
        let mismatch =
            zip(expected, actual)
            .map(|(&a, &(b, _))| (a, b))
            .filter(|(a, b)| a != b)
            .next();
        if let Some((exp, act)) = mismatch {
            let err = if exp < act { ValidationError::MissingClue(exp) }
                else { ValidationError::ExtraClue(act) };
            return Err(err)
        }
        Ok(())
    }

    /// Given the shape of the grid, these are the numbers of each clue.
    fn expected_grid_nums(&self) -> (Vec<u16>, Vec<u16>) {
        let width = self.width as usize;
        let mut across = Vec::new();
        let mut down = Vec::new();
        let mut num = 1;
        for (idx, cell) in self.grid.iter().enumerate() {
            if cell.is_wall() { continue; }
            let x = idx % width;
            let y = idx / width;
            let is_across = x == 0 || self.grid[idx - 1].is_wall();
            let is_down = y == 0 || self.grid[idx - width].is_wall();
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
