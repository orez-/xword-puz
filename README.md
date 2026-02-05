# xword-puz

A Rust wasm library for generating `.puz` crossword files.

## Usage (Rust library)

```
cargo add --git https://github.com/orez-/xword-puz.git
```

```rust
let format = FileFormat::Puz12;
let xword = CrosswordArgs {
    width, height, grid,
    title, author, copyright, notes,
    across_clues, down_clues,
};
let puz_contents = xword.validate()?.export(format)?;
```

## Usage (wasm library)

```
npm install --save https://github.com/orez-/xword-puz/releases/download/0.2.0/xword-puz-0.2.0.tgz
```

```js
import init, { generate_puz } from "xword-puz";
await init();

const format = "puz1.2"; // one of "puz1.2", "puz2.0", or "ipuz"

const puzContents = generate_puz({
    width, height, grid,
    title, author, copyright, notes,
    acrossClues, downClues,
}, format);
```

## `CrosswordArgs`

- `grid` is a list of fill for crossword cells, represented left to right, top to bottom.
  - In javascript, a crossword cell is either a string of its fill, or `null` for walls.
  - `grid` must contain exactly `width * height` elements.
- `acrossClues` and `downClues` are lists of `number, clue` pairs.
- `title`, `author`, `copyright`, and `notes` are strings of metadata about the puzzle.
