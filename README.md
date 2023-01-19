# img2puz

A tool for converting clues and an image of a crossword grid into a .puz crossword file.

[See it in action!](https://orez-.github.io/img2puz/)

## Who this is for
If you have an image of a blank crossword grid, along with its textual, numbered clues, this tool will convert it to a common format that can be shared and solved with solving software such as Across Lite.

Note that this software will _not_ set the puzzle solution, so your puzzle-solving tool will be unable to confirm your solve.

## Who this is NOT for
This is NOT a tool for _setting_ a crossword.
There exist dedicated tools for setting black vs white squares and their solutions which are better suited to this task.

## Running locally
### Via Compiled Code
The code for the img2puz tool all runs as a static site through the browser, so you can run a local copy by running a webserver for the code in the [`gh-pages` branch](https://github.com/orez-/img2puz/tree/gh-pages).

```sh
git clone -b gh-pages git@github.com:orez-/img2puz.git
python3 -m http.server  # or any webserver
# the tool should be available at http://[::]:8000/ ,
# or whatever python says.
```

### Build from Source
The [`scripts/build.sh`](scripts/build.sh) script will compile the WebASM code into `www/pkg/`.

After [installing Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```sh
./scripts/build.sh
cd www
python3 -m http.server  # or any webserver
# the tool should be available at http://[::]:8000/ ,
# or whatever python says.
```
