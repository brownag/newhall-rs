#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
R_PKG_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${R_PKG_DIR}/.." && pwd)"

SRC_CORE="${REPO_ROOT}/crates/newhall-core"
DST_CORE="${R_PKG_DIR}/src/rust/newhall-core"

if [[ ! -f "${SRC_CORE}/Cargo.toml" ]] || [[ ! -d "${SRC_CORE}/src" ]]; then
  echo "Error: expected crates/newhall-core at ${SRC_CORE}" >&2
  exit 1
fi

rm -rf "${DST_CORE}"
mkdir -p "${DST_CORE}"
cp "${SRC_CORE}/Cargo.toml" "${DST_CORE}/Cargo.toml"
cp -R "${SRC_CORE}/src" "${DST_CORE}/src"
rm -rf "${DST_CORE}/tests" "${DST_CORE}/target"
