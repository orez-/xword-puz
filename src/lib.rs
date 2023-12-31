mod generate_puz;
mod multi_error;

use std::iter::zip;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_wasm_bindgen::Error as SerdeError;
use wasm_bindgen::prelude::*;
pub use crate::multi_error::MultiError;

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
                    (Some(c), _) => CrosswordCell::Char(c),
                }
            }
            None => CrosswordCell::Wall,
        })
    }
}

#[wasm_bindgen]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crossword {
    width: u8,
    height: u8,
    grid: Vec<CrosswordCell>,
    across_clues: Vec<(u16, String)>,
    down_clues: Vec<(u16, String)>,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    copyright: String,
    #[serde(default)]
    notes: String,
}

impl Crossword {
    pub fn validate(&self) -> Result<(), MultiError> {
        let mut issues = MultiError::new();
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

impl TryFrom<JsValue> for Crossword {
    type Error = SerdeError;

    fn try_from(val: JsValue) -> Result<Self, Self::Error> {
        let this: Self = serde_wasm_bindgen::from_value(val)?;

        // Quick catastrophe check.
        // This _could_ live in `validate` instead of being a hard error,
        // but the dimensions not matching the grid data is a pretty
        // fundamental issue.
        let expected_len = this.width as usize * this.height as usize;
        if this.grid.len() != expected_len {
            return Err(SerdeError::invalid_length(
                this.grid.len(),
                &"exactly `width * height` grid entries"
            ));
        }
        Ok(this)
    }
}

#[wasm_bindgen]
pub fn generate_puz(blob: JsValue) -> Result<Vec<u8>, MultiError> {
    let xword: Crossword = blob.try_into()
        .expect("js object should be well-formed");
    xword.validate()?;
    Ok(xword.as_puz())
}

// ===

/// Simple data struct for the crossword object.
/// Can be converted into a `Crossword`.
pub struct CrosswordArgs {
    pub width: u8,
    pub height: u8,
    pub grid: Vec<CrosswordCell>,
    pub across_clues: Vec<(u16, String)>,
    pub down_clues: Vec<(u16, String)>,
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub notes: String,
}

impl From<CrosswordArgs> for Crossword {
    fn from(args: CrosswordArgs) -> Crossword {
        Crossword {
            width: args.width,
            height: args.height,
            grid: args.grid,
            across_clues: args.across_clues,
            down_clues: args.down_clues,
            title: args.title,
            author: args.author,
            copyright: args.copyright,
            notes: args.notes,
        }
    }
}
