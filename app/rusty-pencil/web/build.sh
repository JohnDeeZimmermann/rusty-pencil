#!/usr/bin/env bash
# Builds the browser version of rusty-pencil into a static folder (default target/web).
#
#   app/rusty-pencil/web/build.sh [--out DIR] [--dev]
#
# Needs the wasm32-unknown-unknown target and the wasm-bindgen CLI at the version in Cargo.lock;
# runs wasm-opt when it is on PATH. Follows rune's scripts/web-build.sh, plus a temporary rune
# patch (rune-live-wasm.patch) applied to a copy of the pinned checkout. Native builds never see
# the patch.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
out=$root/target/web
profile=release
while (($#)); do
    case "$1" in
        --out) out=$(realpath -m "${2:?--out needs a directory}"); shift 2 ;;
        --dev) profile=dev; shift ;;
        *) echo "usage: build.sh [--out DIR] [--dev]" >&2; exit 2 ;;
    esac
done
cd "$root"
cargo=${CARGO:-cargo}
# A separate target folder keeps wasm builds from invalidating native ones.
export CARGO_TARGET_DIR=$root/target/web-build

version=$(python3 -c 'import sys,tomllib; print(next(p["version"] for p in tomllib.load(open(sys.argv[1],"rb"))["package"] if p["name"]=="wasm-bindgen"))' Cargo.lock)
if [[ $(wasm-bindgen --version 2>/dev/null) != "wasm-bindgen $version" ]]; then
    echo "Install the matching CLI: cargo install wasm-bindgen-cli --version $version --locked" >&2
    exit 2
fi

# Copy the pinned rune checkout and patch it, once per rune rev.
"$cargo" fetch --target wasm32-unknown-unknown --quiet
manifest=$("$cargo" metadata --format-version 1 --filter-platform wasm32-unknown-unknown | python3 -c '
import json,sys; m=json.load(sys.stdin); print(next(p["manifest_path"] for p in m["packages"] if p["name"]=="rune"))')
checkout=$(dirname "$(dirname "$(dirname "$manifest")")")
# Outside the workspace: cargo would make a path dependency under it a workspace member.
rune=${XDG_CACHE_HOME:-$HOME/.cache}/rusty-pencil/web-rune/$(basename "$checkout")
if [[ ! -f $rune/.patched ]]; then
    rm -rf "$rune"
    mkdir -p "$(dirname "$rune")"
    cp -r "$checkout" "$rune"
    patch -d "$rune" -p1 --quiet < "$here/rune-live-wasm.patch"
    touch "$rune/.patched"
fi
source=https://github.com/anomalyco/rune
build=(build -p rusty-pencil --bin rusty-pencil --target wasm32-unknown-unknown
    --config "patch.\"$source\".rune.path=\"$rune/crates/rune\""
    --config "patch.\"$source\".rune-kit.path=\"$rune/crates/rune-kit\"")
if [[ $profile == release ]]; then
    build+=(--release)
    # rune's web-build.sh settings: size over speed, one codegen unit, no debug info.
    export CARGO_PROFILE_RELEASE_OPT_LEVEL=s CARGO_PROFILE_RELEASE_LTO=thin \
        CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_DEBUG=0 \
        CARGO_PROFILE_RELEASE_STRIP=debuginfo
    artifact=$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/rusty-pencil.wasm
else
    artifact=$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/rusty-pencil.wasm
fi
"$cargo" "${build[@]}"

next=$out/.next
rm -rf "$next"
mkdir -p "$next"
wasm-bindgen --target web --out-dir "$next" --out-name app "$artifact"
if [[ $profile == release ]] && command -v wasm-opt >/dev/null; then
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
        --enable-reference-types "$next/app_bg.wasm" -o "$next/app_bg.opt.wasm"
    mv "$next/app_bg.opt.wasm" "$next/app_bg.wasm"
fi
cp "$here/index.html" "$here/favicon.svg" "$next/"
# Move each file into place, so a page loading mid-build never mixes two builds.
for entry in "$next"/*; do
    [[ -d $entry ]] && rm -rf "${out:?}/$(basename "$entry")"
    mv -f "$entry" "$out/"
done
rmdir "$next"
python3 -c 'import gzip,pathlib,sys; p=pathlib.Path(sys.argv[1]); b=p.read_bytes(); print(f"{p}: {len(b):,} bytes raw, {len(gzip.compress(b,mtime=0)):,} bytes gzip")' "$out/app_bg.wasm"
echo "Serve with: python3 -m http.server 8000 -d $out  (then open http://localhost:8000)"
