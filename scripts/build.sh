set -euxo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "${SCRIPT_DIR}/.."

wasm-pack build --target web

# https://github.com/rustwasm/wasm-pack/issues/1039
# https://github.com/rustwasm/wasm-pack/pull/1061
# js packaging is hell and wasm-pack isn't helping.
awk -i inplace 'NR==1{print; print "  \"type\": \"module\","} NR!=1' pkg/package.json

wasm-pack pack
