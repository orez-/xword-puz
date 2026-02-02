use crate::{Crossword, CrosswordCell, EncodingError};
use packed_struct::prelude::*;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::from_fn;

// The lifecycle for a crossword is:
// - `CrosswordArgs`: simple, unvalidated container of named fields
//   for the user to populate.
// - `CrosswordArgs::validate` -> `Crossword`: validated version of the crossword.
// - `Crossword::preserialize` -> `PreserializedCrossword`: intermediate struct
//   of the data in a format more closely representative of the bytes in the `.puz`.
//   We convert some of these into the `Header`, and use some of them directly in
//   the `.puz` bytes.
// - `Header::new`: generated from the `PreserializedCrossword`. These bytes are
//   directly plopped into the start of the `.puz` file.

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct Header {
    checksum: u16,
    file_magic: [u8; 12],
    cib_checksum: u16,
    masked_checksums: [u8; 8],
    version_string: [u8; 4],
    reserved_1c: u16,
    scrambled_checksum: u16,
    reserved_20: [u8; 12],
    width: u8,
    height: u8,
    clue_count: u16,
    unknown_bitmask: u16,
    scrambled_tag: u16,
}

impl Header {
    fn new(crossword: &PreserializedCrossword) -> Self {
        let mut this = Self {
            checksum: 0,
            file_magic: *b"ACROSS&DOWN\0",
            cib_checksum: 0,
            masked_checksums: *b"ICHEATED",
            version_string: crossword.version,
            reserved_1c: 0,
            scrambled_checksum: 0,
            reserved_20: [0; 12],
            width: crossword.width,
            height: crossword.height,
            clue_count: crossword.clues.len() as u16,
            unknown_bitmask: 1, // Set to match oracle, not by the spec.
            scrambled_tag: 0,
        };
        this.generate_checksums(crossword);
        this
    }

    fn generate_checksums(&mut self, crossword: &PreserializedCrossword) {
        let packed = self.pack().unwrap();
        self.cib_checksum = cksum_region(&packed[0x2C..0x34], 0);

        let mut cksum = self.cib_checksum;
        cksum = cksum_region(&crossword.solution, cksum);
        cksum = cksum_region(&crossword.grid, cksum);
        cksum = Self::generate_meta_checksum(crossword, cksum);
        self.checksum = cksum;

        let [cib_high, cib_low] = self.cib_checksum.to_be_bytes();
        let [soln_high, soln_low] = cksum_region(&crossword.solution, 0).to_be_bytes();
        let [grid_high, grid_low] = cksum_region(&crossword.grid, 0).to_be_bytes();
        let [meta_high, meta_low] = Self::generate_meta_checksum(crossword, 0).to_be_bytes();
        self.masked_checksums = [
            b'I' ^ cib_low,
            b'C' ^ soln_low,
            b'H' ^ grid_low,
            b'E' ^ meta_low,
            b'A' ^ cib_high,
            b'T' ^ soln_high,
            b'E' ^ grid_high,
            b'D' ^ meta_high,
        ];
    }

    fn generate_meta_checksum(crossword: &PreserializedCrossword, initial: u16) -> u16 {
        let mut cksum = initial;

        // we need to hash the null terminator on these meta fields
        if !crossword.title.is_empty() {
            cksum = cksum_region(&crossword.title, cksum);
            cksum = cksum_region(&[0], cksum);
        }

        if !crossword.author.is_empty() {
            cksum = cksum_region(&crossword.author, cksum);
            cksum = cksum_region(&[0], cksum);
        }

        if !crossword.copyright.is_empty() {
            cksum = cksum_region(&crossword.copyright, cksum);
            cksum = cksum_region(&[0], cksum);
        }

        for clue in &crossword.clues {
            // weirdly, we DONT hash the null terminator for the clues
            cksum = cksum_region(clue, cksum);
        }

        if !crossword.notes.is_empty() {
            cksum = cksum_region(&crossword.notes, cksum);
            cksum = cksum_region(&[0], cksum);
        }
        cksum
    }
}

fn cksum_region(base: &[u8], mut cksum: u16) -> u16 {
    for &byte in base {
        if cksum & 1 == 1 {
            cksum >>= 1;
            cksum += 0x8000;
        } else {
            cksum >>= 1;
        }
        cksum = cksum.wrapping_add(byte as u16);
    }
    cksum
}

/// Data about the crossword in a format that more closely matches
/// the format used in the `.puz` file.
struct PreserializedCrossword<'a> {
    width: u8,
    height: u8,
    solution: Vec<u8>,
    grid: Vec<u8>,
    clues: Vec<Cow<'a, [u8]>>,
    title: Cow<'a, [u8]>,
    author: Cow<'a, [u8]>,
    copyright: Cow<'a, [u8]>,
    notes: Cow<'a, [u8]>,
    version: [u8; 4],
}

impl Crossword {
    fn preserialize(&self, version: [u8; 4]) -> Result<PreserializedCrossword<'_>, EncodingError> {
        // As near as I can tell, version 2.0 is identical to 1.2,
        // except for the encoding.
        // Excited to be proven wrong about this immediately ᖍ(∙⥚∙)ᖌ
        let encoding = match &version {
            b"1.2\0" => encoding_rs::WINDOWS_1252,
            b"2.0\0" => encoding_rs::UTF_8,
            _ => panic!(),
        };
        let solution = self
            .grid
            .iter()
            .map(|cell| match cell {
                CrosswordCell::Char(c) => *c as u8,
                CrosswordCell::Rebus(s) => s.bytes().next().expect("rebus may not be empty"),
                CrosswordCell::Wall => b'.',
                CrosswordCell::Empty => b'A', // XXX: ???
            })
            .collect();

        let grid = self
            .grid
            .iter()
            .map(|cell| match cell {
                CrosswordCell::Wall => b'.',
                _ => b'-',
            })
            .collect();

        // Clues are represented as a single list with no numbers.
        // Numbers are inferred from the shape of the grid.
        // Both Across and Down clues are intermingled(!?): the clues
        // are in numeric order, favoring Across.
        fn augment<'a, F: Fn(u16) -> String + 'a>(clues: &'a [(u16, String)], f: F) -> impl Iterator<Item = (u16, &'a str, String)> + 'a {
            clues.iter().map(move |&(n, ref c)| (n, c.as_str(), f(n)))
        }

        let across = augment(&self.across_clues, |n| format!("clue {n}A"));
        let down = augment(&self.down_clues, |n| format!("clue {n}D"));
        let clues: Result<Vec<_>, _> = merge_by(across, down, |a, d| a.0.cmp(&d.0))
            .map(|(_, clue, field)| encode(encoding, clue, &field))
            .collect();
        let clues = clues?;

        let xword = PreserializedCrossword {
            width: self.width,
            height: self.height,
            solution,
            grid,
            clues,
            title: encode(encoding, &self.title, "title")?,
            author: encode(encoding, &self.author, "author")?,
            copyright: encode(encoding, &self.copyright, "copyright")?,
            notes: encode(encoding, &self.notes, "notes")?,
            version,
        };
        Ok(xword)
    }

    pub(crate) fn to_puz(&self, version: [u8; 4]) -> Result<Vec<u8>, EncodingError> {
        let this = self.preserialize(version)?;
        let mut puz = Header::new(&this).pack().unwrap().to_vec();
        puz.extend(this.solution);
        puz.extend(this.grid);

        let lines = [&this.title, &this.author, &this.copyright]
            .into_iter()
            .chain(&this.clues)
            .chain([&this.notes]);
        for line in lines {
            puz.extend(line.iter());
            puz.push(0);
        }
        puz.extend(build_rebus_sections(self));
        Ok(puz)
    }
}

fn encode<'a>(encoding: &'static encoding_rs::Encoding, string: &'a str, field: &str) -> Result<Cow<'a, [u8]>, EncodingError> {
    // learning three years later that encoding_rs is specifically
    // for web encoding, and thus silently encodes unrepresentable characters
    // as an html entity. It denotes that it did this with a `bool` in the
    // return tuple, like a Go library.
    let (out, _, failed) = encoding.encode(string);
    if failed {
        // TODO: might be nice to use an encoding library that told you where the issue is,
        // so you could pass that along as feedback to the user.
        // Right now I'd rather the devil I know.
        return Err(EncodingError { field: field.to_owned() })
    }
    Ok(out)
}

/// Merge two iterables (each sorted by `cmp`) into a single `cmp`-sorted iterator.
///
/// When two elements are equal by `cmp`, prefers `a`.
fn merge_by<A, B, T, F>(a: A, b: B, cmp: F) -> impl Iterator<Item = T>
where
    A: IntoIterator<Item = T>,
    B: IntoIterator<Item = T>,
    F: Fn(&T, &T) -> Ordering,
{
    let mut a = a.into_iter().peekable();
    let mut b = b.into_iter().peekable();
    from_fn(move || {
        let which = match (a.peek(), b.peek()) {
            (Some(left), Some(right)) => cmp(left, right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => return None,
        };

        match which {
            Ordering::Less => a.next(),
            Ordering::Equal => a.next(),
            Ordering::Greater => b.next(),
        }
    })
}

fn build_rebus_sections(xword: &Crossword) -> Vec<u8> {
    let mut max_rebus = 0; // the musician?
    let mut seen_rebus: HashMap<&str, u8> = HashMap::new();
    let mut rebus_words = Vec::new();

    let rebus_grid: Vec<_> = xword
        .grid
        .iter()
        .map(|cell| match cell {
            CrosswordCell::Rebus(s) => *seen_rebus.entry(s).or_insert_with(|| {
                rebus_words.extend(format!("{max_rebus:>2}:{s};").bytes());
                max_rebus += 1;
                max_rebus
            }),
            _ => 0,
        })
        .collect();

    let mut out = Vec::new();
    if seen_rebus.is_empty() {
        return out;
    }
    out.extend(extra_section(*b"GRBS", &rebus_grid));
    out.extend(extra_section(*b"RTBL", &rebus_words));
    out
}

fn extra_section(title: [u8; 4], data: &[u8]) -> Vec<u8> {
    let len = data.len() as u16;
    let checksum = cksum_region(data, 0);
    let mut out = Vec::new();
    out.extend(title);
    out.extend(len.to_le_bytes());
    out.extend(checksum.to_le_bytes());
    out.extend(data);
    out.push(0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CrosswordArgs;

    #[test]
    fn test_smol_rebus() {
        let xword = CrosswordArgs {
            width: 2,
            height: 2,
            grid: vec![
                CrosswordCell::Rebus("ON".to_string()), CrosswordCell::Rebus("TO".to_string()),
                CrosswordCell::Rebus("LY".to_string()), CrosswordCell::Rebus("ON".to_string()),
            ],
            across_clues: vec![(1, "Aware of".to_string()), (3, "French city".to_string())],
            down_clues: vec![(1, "Solely".to_string()), (2, "Animated sort".to_string())],
            title: "smol".to_string(),
            author: "me".to_string(),
            copyright: String::new(),
            notes: String::new(),
        };
        let xword = xword.validate().unwrap();
        let puz = xword.to_puz(*b"2.0\0").unwrap();

        assert_eq!(puz, include_bytes!("test_files/smol.puz"));
    }

    #[test]
    fn test_encoding_oracle() {
        let xword = CrosswordArgs {
            width: 5,
            height: 5,
            grid: vec![
                'A', 'A', 'H', 'E', 'D',
                'A', 'N', 'A', 'I', 'S',
                'B', 'O', 'R', 'E', 'S',
                'B', 'A', 'T', 'I', 'N',
                'A', 'S', 'E', 'A', 'T'
            ].into_iter().map(CrosswordCell::Char).collect(),
            across_clues: vec![
                (1, "no".to_string()),
                (6, "no".to_string()),
                (7, "no".to_string()),
                (8, "no".to_string()),
                (9, "no".to_string()),
            ],
            down_clues: vec![
                (1, "no".to_string()),
                (2, "no".to_string()),
                (3, "no".to_string()),
                (4, "no".to_string()),
                (5, "no".to_string()),
            ],
            title: "🫛 Test".to_string(),
            author: "Anonymous".to_string(),
            copyright: "Copyright Anonymous, all rights reserved".to_string(),
            notes: "Created on crosshare.org".to_string(), // lol
        };
        let xword = xword.validate().unwrap();
        let puz = xword.to_puz(*b"2.0\0").unwrap();

        assert_eq!(puz, include_bytes!("test_files/encoding_oracle.puz"));
    }

    #[test]
    fn test_one_long() {
        // one-long areas do NOT get clues.
        let xword = CrosswordArgs {
            width: 2,
            height: 2,
            grid: vec![
                CrosswordCell::Char('A'), CrosswordCell::Char('B'),
                CrosswordCell::Char('C'), CrosswordCell::Wall,
            ],
            across_clues: vec![(1, "Layout testing strategy".to_string())],
            down_clues: vec![(1, "Initials in cooling".to_string())],
            title: "one long".to_string(),
            author: "me".to_string(),
            copyright: String::new(),
            notes: String::new(),
        };
        let xword = xword.validate().unwrap();
        let _puz = xword.to_puz(*b"2.0\0").unwrap();
    }
}
