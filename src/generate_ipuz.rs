// "version": "http://ipuz.org/v2",
// "kind": [ "http://ipuz.org/crossword#1" ],
// "dimensions": { "Dimension": n, ... },
// "puzzle": [ [ LabeledCell, ... ], ... ],
// "solution": [ [ CrosswordValue, ... ], ... ],
// "clues": { "Across": [ Clue, ... ],
//            "Down": [ Clue, ... ] },

// The ipuz format is the 2010s-in'est format you ever saw.
// Big ol' open-ended json blob.
// I do not care for it.

use crate::lit_str;
use crate::multi_error::MultiError;
use crate::validation::{ClueError, validate_clues};
use crate::{Crossword, CrosswordCell, Grid, NumberedCell};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::iter::zip;

lit_str!(Version, "http://ipuz.org/v1");
lit_str!(Kind, "http://ipuz.org/crossword#1");

type ClueList = Vec<(u16, String)>;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Clues<'a> {
    #[serde(borrow)]
    across: Cow<'a, ClueList>,
    #[serde(borrow)]
    down: Cow<'a, ClueList>,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
struct Dimensions {
    width: u8,
    height: u8,
}

#[derive(thiserror::Error, Debug)]
enum DeserializeError {
    #[error("expected {expected} clues, found {actual}")]
    MismatchedClueCount { expected: usize, actual: usize },
    #[error("found misordered clues. Clue numbers must be strictly increasing")]
    MisorderedClues,
    #[error("missing clue #{0}")]
    MissingClue(u16),
    #[error("found extraneous clue #{0}")]
    ExtraClue(u16),
    #[error("grid is height {height}, but found {actual} rows")]
    InvalidHeight { height: usize, actual: usize },
    #[error("grid is width {width}, but row {row} is length {actual}")]
    InvalidWidth {
        row: usize,
        width: usize,
        actual: usize,
    },
    #[error(
        "invalid solution item at {row},{col}: expected string or block ({block}), but found {actual}"
    )]
    InvalidSolutionItem {
        row: usize,
        col: usize,
        block: StringOrNum,
        actual: StringOrNum,
    },
    #[error("invalid numbering at {row},{col}: expected {expected} but found {actual}")]
    InvalidNumbering {
        row: usize,
        col: usize,
        expected: LabeledCellValue,
        actual: LabeledCellValue,
    },
    #[error("error in labeled cell at {row},{col}: {error}")]
    LabeledCellError {
        row: usize,
        col: usize,
        error: LabeledCellError,
    },
}

impl From<ClueError> for DeserializeError {
    fn from(err: ClueError) -> DeserializeError {
        match err {
            ClueError::MismatchedClueCount { expected, actual } => {
                DeserializeError::MismatchedClueCount { expected, actual }
            }
            ClueError::MisorderedClues => DeserializeError::MisorderedClues,
            ClueError::MissingClue(clue) => DeserializeError::MissingClue(clue),
            ClueError::ExtraClue(clue) => DeserializeError::ExtraClue(clue),
        }
    }
}

fn validate_dimensions<T>(dim: Dimensions, puzzle: &[Vec<T>]) -> Result<(), DeserializeError> {
    let width = dim.width as usize;
    let height = dim.height as usize;
    if puzzle.len() != height {
        let err = DeserializeError::InvalidHeight {
            height,
            actual: puzzle.len(),
        };
        return Err(err);
    }
    let err = puzzle.iter().enumerate().find_map(|(row, r)| {
        (r.len() != width).then_some(DeserializeError::InvalidWidth {
            row,
            width,
            actual: r.len(),
        })
    });
    // wish there were an idiom for `Option<E>` -> `Result<(), E>`
    if let Some(err) = err {
        return Err(err);
    }
    Ok(())
}

/// Representation of the ipuz file which closely mirrors the json.
/// As such it is not validated, nor is it represented in a Rust-ily ergonomic way
/// for manipulation/processing. Used as the serde layer.
#[derive(Deserialize, Serialize)]
struct IPuzRaw<'a> {
    version: Version,
    kind: [Kind; 1],
    title: &'a str,
    copyright: &'a str,
    author: &'a str,
    notes: &'a str,
    dimensions: Dimensions,
    #[serde(default = "default_block")]
    block: StringOrNum,
    #[serde(default = "default_empty")]
    empty: StringOrNum,
    puzzle: Vec<Vec<LabeledCell>>,
    solution: Vec<Vec<CrosswordValue>>,
    clues: Clues<'a>,
}

fn default_block() -> StringOrNum {
    StringOrNum::String("#".into())
}

fn default_empty() -> StringOrNum {
    StringOrNum::Num(0)
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(untagged)]
enum StringOrNum {
    String(String),
    Num(i32),
}

impl fmt::Display for StringOrNum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s:?}"),
            Self::Num(n) => write!(f, "{n:?}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum LabeledCellError {
    #[error("string labels are unsupported (found {0:?})")]
    String(String),
    #[error("numeric label is out of supported range (found {0:?})")]
    Num(i32),
}

// nb this does not cover the possible values of a LabeledCell per the spec,
// but the spec is too open-ended and I do not care right now.
#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum LabeledCell {
    Raw(StringOrNum),
    Cell { cell: StringOrNum },
}

impl LabeledCell {
    fn cell(num: i32) -> Self {
        LabeledCell::Cell {
            cell: StringOrNum::Num(num),
        }
    }

    fn to_value(
        &self,
        block: &StringOrNum,
        empty: &StringOrNum,
    ) -> Result<LabeledCellValue, LabeledCellError> {
        let sorn: &StringOrNum = self.into();
        match sorn {
            sorn if sorn == block => Ok(LabeledCellValue::Block),
            sorn if sorn == empty => Ok(LabeledCellValue::Empty),
            StringOrNum::String(string) => Err(LabeledCellError::String(string.to_owned())),
            &StringOrNum::Num(num) => {
                let num: u16 = num.try_into().map_err(|_| LabeledCellError::Num(num))?;
                Ok(LabeledCellValue::Number(num))
            }
        }
    }
}

impl From<LabeledCell> for StringOrNum {
    fn from(cell: LabeledCell) -> StringOrNum {
        match cell {
            LabeledCell::Raw(sorn) => sorn,
            LabeledCell::Cell { cell } => cell,
        }
    }
}

impl<'a> From<&'a LabeledCell> for &'a StringOrNum {
    fn from(cell: &'a LabeledCell) -> &'a StringOrNum {
        match cell {
            LabeledCell::Raw(sorn) => sorn,
            LabeledCell::Cell { cell } => cell,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LabeledCellValue {
    Block,
    Empty,
    Number(u16),
}

impl From<NumberedCell> for LabeledCellValue {
    fn from(num_cell: NumberedCell) -> LabeledCellValue {
        match num_cell {
            NumberedCell::Wall => LabeledCellValue::Block,
            NumberedCell::Empty => LabeledCellValue::Empty,
            NumberedCell::Numbered { number, .. } => LabeledCellValue::Number(number),
        }
    }
}

impl fmt::Display for LabeledCellValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LabeledCellValue::Block => write!(f, "block"),
            LabeledCellValue::Empty => write!(f, "no label"),
            LabeledCellValue::Number(number) => write!(f, "#{number}"),
        }
    }
}

type CrosswordValue = StringOrNum;

impl<'a> From<&'a Crossword> for IPuzRaw<'a> {
    fn from(xword: &'a Crossword) -> Self {
        let Crossword {
            width,
            height,
            grid,
            across_clues,
            down_clues,
            title,
            author,
            copyright,
            notes,
        } = xword;

        let chunk = *width as usize;

        let empty = default_empty();
        let empty_cell = LabeledCell::Cell {
            cell: empty.clone(),
        };
        let block = default_block();
        let block_cell = LabeledCell::Raw(block.clone());
        let puzzle: Vec<_> = xword
            .grid()
            .iter_numbered()
            .map(|cell| match cell {
                NumberedCell::Wall => block_cell.clone(),
                NumberedCell::Empty => empty_cell.clone(),
                NumberedCell::Numbered { number, .. } => LabeledCell::cell(number.into()),
            })
            .collect();
        let puzzle = puzzle.chunks(chunk).map(|c| c.to_vec()).collect();
        let block_ref = &block;
        let solution: Vec<_> = grid
            .iter()
            .map(move |cell| {
                let s = match cell {
                    CrosswordCell::Empty => String::new(), // XXX: ?
                    CrosswordCell::Char(c) => c.to_string(),
                    CrosswordCell::Rebus(s) => s.to_string(),
                    CrosswordCell::Wall => return block_ref.clone(),
                };
                StringOrNum::String(s)
            })
            .collect();
        let solution = solution.chunks(chunk).map(|c| c.to_vec()).collect();

        IPuzRaw {
            version: Version,
            kind: [Kind],
            title,
            copyright,
            author,
            notes,
            dimensions: Dimensions {
                width: *width,
                height: *height,
            },
            block,
            empty,
            puzzle,
            solution,
            clues: Clues {
                across: Cow::Borrowed(across_clues),
                down: Cow::Borrowed(down_clues),
            },
        }
    }
}

impl<'a> TryFrom<IPuzRaw<'a>> for Crossword {
    type Error = MultiError<DeserializeError>;

    fn try_from(ipuz: IPuzRaw<'a>) -> Result<Self, Self::Error> {
        let IPuzRaw {
            version: _,
            kind: _,
            title,
            copyright,
            author,
            notes,
            dimensions,
            block,
            empty,
            puzzle,
            solution,
            clues: Clues { across, down },
        } = ipuz;
        let mut issues = MultiError::new();

        if let Err(err) = validate_dimensions(dimensions, &puzzle) {
            issues.insert("puzzle", err);
        }
        if let Err(err) = validate_dimensions(dimensions, &solution) {
            issues.insert("solution", err);
        }

        // short circuiting here: the rest of this code assumes these grids are the right size.
        if !issues.is_empty() {
            return Err(issues);
        }

        let width = dimensions.width as usize;
        let raw_grid: Result<Vec<_>, _> = solution
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(idx, elem)| {
                if elem == block {
                    return Ok(CrosswordCell::Wall);
                }
                let StringOrNum::String(elem) = elem else {
                    let err = DeserializeError::InvalidSolutionItem {
                        row: idx / width,
                        col: idx % width,
                        block: block.clone(),
                        actual: elem,
                    };
                    return Err(err);
                };
                // XXX: we don't currently support non-ascii-alphabetical.
                // if we did, we'd need to rethink this bytesy splat.
                //
                // ...we also don't ever validate that the fill is ascii, and really,
                // TODO: we should.
                let cell = match elem.as_bytes() {
                    [] => CrosswordCell::Empty,
                    &[b] => CrosswordCell::Char(b as char),
                    _ => CrosswordCell::Rebus(elem.to_owned()),
                };
                Ok(cell)
            })
            .collect();
        let raw_grid = match raw_grid {
            Ok(g) => g,
            Err(err) => {
                issues.insert("solution", err);
                return Err(issues);
            }
        };

        let grid = Grid {
            width: dimensions.width,
            height: dimensions.height,
            grid: &raw_grid,
        };

        let puzzle = puzzle.into_iter().flatten();
        let puzzle_error = zip(grid.iter_numbered(), puzzle).enumerate().try_for_each(
            |(idx, (num_cell, lab_cell))| {
                let lab_cell = lab_cell.to_value(&block, &empty).map_err(|error| {
                    DeserializeError::LabeledCellError {
                        row: idx / width,
                        col: idx % width,
                        error,
                    }
                })?;
                let num_cell = num_cell.into();
                if lab_cell != num_cell {
                    let err = DeserializeError::InvalidNumbering {
                        row: idx / width,
                        col: idx % width,
                        expected: num_cell,
                        actual: lab_cell,
                    };
                    return Err(err);
                }
                Ok(())
            },
        );
        if let Err(error) = puzzle_error {
            issues.insert("puzzle", error);
        }

        let (exp_across, exp_down) = grid.expected_grid_nums();
        if let Err(err) = validate_clues(&exp_across, &across) {
            issues.insert("clues.Across", err.into());
        }
        if let Err(err) = validate_clues(&exp_down, &down) {
            issues.insert("clues.Down", err.into());
        }

        if !issues.is_empty() {
            return Err(issues);
        }

        let xword = Crossword {
            title: title.to_owned(),
            copyright: copyright.to_owned(),
            author: author.to_owned(),
            notes: notes.to_owned(),
            width: dimensions.width,
            height: dimensions.height,
            across_clues: across.to_vec(),
            down_clues: down.to_vec(),
            grid: raw_grid,
        };
        Ok(xword)
    }
}

impl Crossword {
    pub fn to_ipuz(&self) -> Vec<u8> {
        let ipuz: IPuzRaw = self.into();
        serde_json::to_vec(&ipuz).expect("serializable") // TODO: don't panic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser() {
        let ipuz = include_str!("test_files/Ups and Downs.ipuz");
        let ipuz: IPuzRaw = serde_json::from_str(ipuz).unwrap();
        let xword: Crossword = ipuz.try_into().unwrap();
        xword.to_ipuz();
    }
}
