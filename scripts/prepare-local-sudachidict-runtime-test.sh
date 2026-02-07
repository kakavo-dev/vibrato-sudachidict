#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_SCRIPT="${REPO_ROOT}/scripts/build-vibrato-sudachidict.sh"
DIST_DIR="${REPO_ROOT}/dist"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/tools/sudachi-vibrato-converter/target/local-sudachidict}"
ASSET_PATH="${ASSET_PATH:-}"
SKIP_BUILD="${SKIP_BUILD:-0}"

if [[ -z "${ASSET_PATH}" && "${SKIP_BUILD}" != "1" ]]; then
  if [[ ! -x "${BUILD_SCRIPT}" ]]; then
    chmod +x "${BUILD_SCRIPT}"
  fi

  echo "[local-test] build sudachidict with vibrato"
  "${BUILD_SCRIPT}"
fi

if [[ -z "${ASSET_PATH}" ]]; then
  shopt -s nullglob
  assets=("${DIST_DIR}"/sudachidict-*-full+vibrato-v0_5_2.tar.xz)
  shopt -u nullglob
  if (( ${#assets[@]} == 0 )); then
    echo "[error] no dist asset found under ${DIST_DIR}" >&2
    echo "hint: run without SKIP_BUILD=1 or pass ASSET_PATH=/absolute/path/to/sudachidict-*.tar.xz" >&2
    exit 1
  fi
  ASSET_PATH="$(ls -1t "${assets[@]}" | head -n 1)"
fi

if [[ ! -f "${ASSET_PATH}" ]]; then
  echo "[error] asset file not found: ${ASSET_PATH}" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/local-sudachidict-runtime.XXXXXX")"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "[local-test] extract asset: ${ASSET_PATH}"
tar -xJf "${ASSET_PATH}" -C "${TMP_DIR}"

SYSTEM_DIC_SRC="$(find "${TMP_DIR}" -type f -name 'system.dic.zst' | head -n 1)"
METADATA_SRC="$(find "${TMP_DIR}" -type f -name 'metadata.json' | head -n 1)"
if [[ -z "${SYSTEM_DIC_SRC}" ]]; then
  echo "[error] system.dic.zst was not found in asset: ${ASSET_PATH}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"
cp "${SYSTEM_DIC_SRC}" "${OUT_DIR}/system.dic.zst"
if [[ -n "${METADATA_SRC}" ]]; then
  cp "${METADATA_SRC}" "${OUT_DIR}/metadata.json"
fi

cat > "${OUT_DIR}/README.local-test.txt" <<EOF
This directory is managed by:
${REPO_ROOT}/scripts/prepare-local-sudachidict-runtime-test.sh

system.dic.zst is copied from:
${ASSET_PATH}
EOF

echo "[local-test] installed dictionary:"
echo "  ${OUT_DIR}/system.dic.zst"
if [[ -f "${OUT_DIR}/metadata.json" ]]; then
  echo "  ${OUT_DIR}/metadata.json"
fi
