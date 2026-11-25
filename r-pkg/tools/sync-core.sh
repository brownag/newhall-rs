#!/usr/bin/env bash
set -euo pipefail

MODE="sync"
if [[ $# -gt 1 ]]; then
  echo "Usage: $0 [--check]" >&2
  exit 2
elif [[ $# -eq 1 ]]; then
  case "$1" in
    --check)
      MODE="check"
      ;;
    *)
      echo "Usage: $0 [--check]" >&2
      exit 2
      ;;
  esac
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
R_PKG_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${R_PKG_DIR}/.." && pwd)"

SRC_CORE="${REPO_ROOT}/crates/newhall-core"
DST_CORE="${R_PKG_DIR}/src/rust/newhall-core"

if [[ ! -f "${SRC_CORE}/Cargo.toml" ]] || [[ ! -d "${SRC_CORE}/src" ]]; then
  echo "Error: expected crates/newhall-core at ${SRC_CORE}" >&2
  exit 1
fi

if [[ "${MODE}" == "check" ]]; then
  if [[ ! -f "${DST_CORE}/Cargo.toml" ]] || [[ ! -d "${DST_CORE}/src" ]]; then
    echo "Error: vendored newhall-core missing at ${DST_CORE}. Run bash r-pkg/tools/sync-core.sh to sync." >&2
    exit 1
  fi

  DRIFT=0
  if ! diff -u "${SRC_CORE}/Cargo.toml" "${DST_CORE}/Cargo.toml" >&2; then
    DRIFT=1
  fi
  if ! diff -rq "${SRC_CORE}/src" "${DST_CORE}/src" >&2; then
    DRIFT=1
  fi

  if [[ "${DRIFT}" -ne 0 ]]; then
    echo "Error: r-pkg/src/rust/newhall-core has drifted from crates/newhall-core. Run bash r-pkg/tools/sync-core.sh to sync." >&2
    exit 1
  fi

  echo "OK: r-pkg/src/rust/newhall-core matches crates/newhall-core."
  exit 0
fi

rm -rf "${DST_CORE}"
mkdir -p "${DST_CORE}"
cp "${SRC_CORE}/Cargo.toml" "${DST_CORE}/Cargo.toml"
cp -R "${SRC_CORE}/src" "${DST_CORE}/src"
rm -rf "${DST_CORE}/tests" "${DST_CORE}/target"
