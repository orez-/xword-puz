use std::borrow::Cow;
use std::cmp::Ordering;
use std::iter::from_fn;
use packed_struct::prelude::*;
use encoding_rs::WINDOWS_1252;
use crate::{Crossword, CrosswordCell};

#[derive(PackedStruct)]
#[packed_struct(endian="lsb")]
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
            version_string: *b"1.2\0",
            reserved_1c: 0,
            scrambled_checksum: 0,
            reserved_20: [0; 12],
            width: crossword.width,
            height: crossword.height,
            clue_count: crossword.clues.len() as u16,
            unknown_bitmask: 0,
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

        // TODO: i think we gotta hash the null terminator too.
        if !crossword.title.is_empty() {
            cksum = cksum_region(&crossword.title, cksum);
        }

        if !crossword.author.is_empty() {
            cksum = cksum_region(&crossword.author, cksum);
        }

        if !crossword.copyright.is_empty() {
            cksum = cksum_region(&crossword.copyright, cksum);
        }

        for clue in &crossword.clues {
            // XXX: but maybe we dont hash the null terminator for the clues??
            cksum = cksum_region(clue, cksum);
        }

        if crossword.notes.is_empty() {
            cksum = cksum_region(&crossword.notes, cksum);
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
        cksum += byte as u16;
    }
    cksum
}

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
}


impl Crossword {
    fn preserialize(&self) -> PreserializedCrossword<'_> {
        let solution = self.cells.iter().map(|cell| match cell {
            CrosswordCell::Char(c) => *c as u8,
            CrosswordCell::Rebus(s) => s.bytes().next().unwrap(),
            CrosswordCell::Wall => b'.',
        }).collect();

        let grid = self.cells.iter().map(|cell| match cell {
            CrosswordCell::Wall => b'.',
            _ => b'-',
        }).collect();

        let mut across = self.across_clues.iter().peekable();
        let mut down = self.down_clues.iter().peekable();
        let clues = from_fn(|| {
            let which = match (across.peek(), down.peek()) {
                (Some((a, _)), Some((d, _))) => Some(a.cmp(d)),
                (Some(_), None) => Some(Ordering::Less),
                (None, Some(_)) => Some(Ordering::Greater),
                (None, None) => None,
            };

            match which {
                Some(Ordering::Less) => across.next(),
                Some(Ordering::Equal) => across.next(),
                Some(Ordering::Greater) => down.next(),
                None => None,
            }.map(|(_, clue)| WINDOWS_1252.encode(&clue).0)
        }).collect();

        PreserializedCrossword {
            width: self.width,
            height: self.height,
            solution,
            grid,
            clues,
            title: WINDOWS_1252.encode(&self.title).0,
            author: WINDOWS_1252.encode(&self.author).0,
            copyright: WINDOWS_1252.encode(&self.copyright).0,
            notes: WINDOWS_1252.encode(&self.notes).0,
        }
    }

    pub fn as_puz(&self) -> Vec<u8> {
        let this = self.preserialize();
        let mut puz = Header::new(&this).pack().unwrap().to_vec();
        puz.extend(this.solution);
        puz.extend(this.grid);

        let lines = [&this.title, &this.author, &this.copyright].into_iter()
            .chain(&this.clues)
            .chain([&this.notes]);
        for line in lines {
            puz.extend(line.into_iter());
            puz.push(0);
        }
        puz
    }
}
