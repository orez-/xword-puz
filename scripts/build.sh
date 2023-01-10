SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "${SCRIPT_DIR}/.."

mkdir -p www/pkg/
cargo build --release --target wasm32-unknown-unknown
cd target/wasm32-unknown-unknown/release
wasm-bindgen --target web --no-typescript --out-dir . img2puz.wasm
wasm-gc img2puz.wasm
cp img2puz_bg.wasm ../../../www/pkg/
cp img2puz.js ../../../www/pkg/
