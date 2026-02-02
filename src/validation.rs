use std::iter::zip;

#[derive(thiserror::Error, Debug)]
pub(crate) enum ClueError {
    #[error("expected {expected} clues, found {actual}")]
    MismatchedClueCount { expected: usize, actual: usize },
    #[error("found misordered clues. Clue numbers must be strictly increasing")]
    MisorderedClues,
    #[error("missing clue #{0}")]
    MissingClue(u16),
    #[error("found extraneous clue #{0}")]
    ExtraClue(u16),
}

pub(crate) fn validate_clues(expected: &[u16], actual: &[(u16, String)]) -> Result<(), ClueError> {
    if actual.windows(2).any(|w| w[0] >= w[1]) {
        return Err(ClueError::MisorderedClues);
    }
    if expected.len() != actual.len() {
        let expected = expected.len();
        let actual = actual.len();
        return Err(ClueError::MismatchedClueCount { expected, actual });
    }
    let mismatch = zip(expected, actual)
        .map(|(&a, &(b, _))| (a, b))
        .find(|(a, b)| a != b);
    if let Some((exp, act)) = mismatch {
        let err = if exp < act {
            ClueError::MissingClue(exp)
        } else {
            ClueError::ExtraClue(act)
        };
        return Err(err);
    }
    Ok(())
}
