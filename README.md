# xword-puz

A Rust wasm library for generating `.puz` crossword files.

## Usage (Rust library)

```
cargo add --git https://github.com/orez-/xword-puz.git
```

```rust
let xword: Crossword = CrosswordArgs {
    width, height, grid,
    title, author, copyright, notes,
    across_clues, down_clues,
}.into();
xword.validate()?;
let puz_contents = xword.as_puz();
```

## Usage (wasm library)

```
npm install --save https://github.com/orez-/xword-puz/releases/download/0.1.3/xword-puz-0.1.3.tgz
```

```js
import init, { generate_puz } from "xword-puz";
await init();

const puzContents = generate_puz({
    width, height, grid,
    title, author, copyright, notes,
    acrossClues, downClues,
});
```
