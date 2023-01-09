use image::io::Reader as ImageReader;
use image::GenericImageView;
use crate::CrosswordCell;

const DARK_THRESHOLD: u8 = 0x80;
const CLOSE_THRESHOLD: usize = 3;

struct CrosswordDimensions {
    rows: Vec<usize>,
    cols: Vec<usize>,
    cell_size: usize,
    width: usize,
    height: usize,
}

pub struct CrosswordGrid {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<CrosswordCell>,
}

pub fn load_crossword(filename: &str) -> Result<CrosswordGrid, ()> {
    let img = ImageReader::open(filename).unwrap().decode().unwrap();
    let img = img.into_luma8();
    let dims = find_xword_dimensions(&img);

    let mut cells = Vec::with_capacity(dims.width * dims.height);
    let sq = dims.cell_size * dims.cell_size;
    for &row in &dims.rows {
        for &col in &dims.cols {
            let set = img.view(col as u32, row as u32, dims.cell_size as u32, dims.cell_size as u32)
                .pixels()
                .filter(|px| px.2.0[0] <= DARK_THRESHOLD)
                .count();
            let is_wall = set >= sq / 2;
            // println!("[{row},{col}]: {set}/{sq} => {is_wall}");
            cells.push(
                if is_wall { CrosswordCell::Wall }
                else { CrosswordCell::empty() }
            );
        }
    }

    Ok(CrosswordGrid {
        width: dims.width as u8,
        height: dims.height as u8,
        cells,
    })
}

fn find_xword_dimensions(img: &image::GrayImage) -> CrosswordDimensions {
    let longest_black_lines: Vec<_> = img.rows().map(|row| {
        let mut start = None;
        let mut best_start = 0;
        let mut best_end = 0;

        for (x, black) in row.map(|px| px.0[0] <= DARK_THRESHOLD).chain([false]).enumerate() {
            match (black, start) {
                (true, None) => { start = Some(x); }
                (false, Some(st)) => {
                    if best_end - best_start < x - st {
                        best_start = st;
                        best_end = x;
                        start = None;
                    }
                }
                _ => (),
            }
        }
        (best_start, best_end)
    }).collect();
    let &(x0, x1) = longest_black_lines.iter()
        .max_by_key(|&(s, e)| e - s)
        .unwrap();

    let rows = longest_black_lines.iter()
        .enumerate()
        .filter_map(|(y, &(s, e))| (is_close(x0, s) && is_close(x1, e)).then(|| y));

    let mut rows: Vec<usize> = dedup_sequential(rows).collect();
    rows.pop();
    let cell_size = rows.windows(2)
        .map(|v| v[1] - v[0])
        .sum::<usize>() / (rows.len() - 1);
    // XXX: this is gonna be a rounding error disaster. rethink this.
    let mut cols: Vec<usize> = (x0..x1).step_by(cell_size).collect();
    cols.pop();

    CrosswordDimensions {
        cell_size,
        width: cols.len(),
        height: rows.len(),
        rows,
        cols,
    }
}

fn dedup_sequential(mut it: impl Iterator<Item=usize>) -> impl Iterator<Item=usize> {
    let mut first = it.next().unwrap();
    let mut prev = first;
    it.chain([usize::MAX]).filter_map(move |elem| {
        if prev + 1 == elem {
            prev = elem;
            None
        } else {  // agg + output the prev set, and start a new set
            let out = (prev + first) / 2;
            first = elem;
            prev = elem;
            Some(out)
        }
    })
}

fn is_close(a: usize, b: usize) -> bool {
    a.abs_diff(b) <= CLOSE_THRESHOLD
}
