#!/usr/bin/env bash
set -euo pipefail

NEOLOGD_REPO="https://github.com/neologd/mecab-ipadic-neologd.git"
NEOLOGD_BRANCH="master"
VIBRATO_REPO="https://github.com/daac-tools/vibrato.git"
VIBRATO_REF="v0.5.2"

WORK_BASE="$(mktemp -d "${RUNNER_TEMP:-/tmp}/vibrato-ipadic-neologd.XXXXXX")"
NEOLOGD_DIR="${WORK_BASE}/mecab-ipadic-neologd"
NEOLOGD_INSTALL_PREFIX="${WORK_BASE}/neologd-install"
VIBRATO_DIR="${WORK_BASE}/vibrato"

echo "[build] workspace: ${WORK_BASE}"
echo "[build] clone neologd: ${NEOLOGD_REPO}@${NEOLOGD_BRANCH}"
git clone --depth 1 --branch "${NEOLOGD_BRANCH}" "${NEOLOGD_REPO}" "${NEOLOGD_DIR}"

echo "[build] build latest mecab-ipadic-neologd"
mkdir -p "${NEOLOGD_INSTALL_PREFIX}"
(
  cd "${NEOLOGD_DIR}"
  ./bin/install-mecab-ipadic-neologd -n -y -u -p "${NEOLOGD_INSTALL_PREFIX}"
)

SEED_FILE="$(find "${NEOLOGD_DIR}/seed" -maxdepth 1 -type f -name 'mecab-user-dict-seed.*.csv.xz' | LC_ALL=C sort | tail -n 1)"
if [[ -z "${SEED_FILE}" ]]; then
  echo "[error] seed file was not found under ${NEOLOGD_DIR}/seed" >&2
  exit 1
fi

SEED_DATE="$(basename "${SEED_FILE}" | sed -E 's/^mecab-user-dict-seed\.([0-9]{8})\.csv\.xz$/\1/')"
if [[ ! "${SEED_DATE}" =~ ^[0-9]{8}$ ]]; then
  echo "[error] failed to parse SEED_DATE from ${SEED_FILE}" >&2
  exit 1
fi

NEOLOGD_SHA="$(git -C "${NEOLOGD_DIR}" rev-parse HEAD)"
NEOLOGD_SHA7="${NEOLOGD_SHA:0:7}"

NEOLOGD_BUILD_DIR="${NEOLOGD_DIR}/build/mecab-ipadic-2.7.0-20070801-neologd-${SEED_DATE}"
if [[ ! -d "${NEOLOGD_BUILD_DIR}" ]]; then
  echo "[error] built dictionary directory does not exist: ${NEOLOGD_BUILD_DIR}" >&2
  exit 1
fi

LEXICON_PATH="${WORK_BASE}/lex.csv"
mapfile -d '' CSV_FILES < <(find "${NEOLOGD_BUILD_DIR}" -maxdepth 1 -type f -name '*.csv' -print0 | LC_ALL=C sort -z)
if [[ "${#CSV_FILES[@]}" -eq 0 ]]; then
  echo "[error] no csv files were found in ${NEOLOGD_BUILD_DIR}" >&2
  exit 1
fi
for csv in "${CSV_FILES[@]}"; do
  cat "${csv}" >> "${LEXICON_PATH}"
done

echo "[build] clone vibrato: ${VIBRATO_REPO}@${VIBRATO_REF}"
git clone --depth 1 --branch "${VIBRATO_REF}" "${VIBRATO_REPO}" "${VIBRATO_DIR}"

SYSTEM_DIC_PATH="${WORK_BASE}/system.dic.zst"
echo "[build] compile vibrato system dictionary"
cargo run --release --manifest-path "${VIBRATO_DIR}/Cargo.toml" -p compile -- \
  -l "${LEXICON_PATH}" \
  -m "${NEOLOGD_BUILD_DIR}/matrix.def" \
  -u "${NEOLOGD_BUILD_DIR}/unk.def" \
  -c "${NEOLOGD_BUILD_DIR}/char.def" \
  -o "${SYSTEM_DIC_PATH}"

echo "[build] smoke test"
printf '%s\n' "vibrato ipadic neologd smoke test" | \
  cargo run --release --manifest-path "${VIBRATO_DIR}/Cargo.toml" -p tokenize -- \
    -i "${SYSTEM_DIC_PATH}" >/dev/null

ASSET_NAME="ipadic-neologd-${SEED_DATE}+n${NEOLOGD_SHA7}+vibrato-v0_5_2.tar.xz"
BUNDLE_DIR_NAME="ipadic-neologd-${SEED_DATE}+n${NEOLOGD_SHA7}+vibrato-v0_5_2"
DIST_ROOT="${WORK_BASE}/dist"
BUNDLE_DIR="${DIST_ROOT}/${BUNDLE_DIR_NAME}"
mkdir -p "${BUNDLE_DIR}"

cp "${SYSTEM_DIC_PATH}" "${BUNDLE_DIR}/system.dic.zst"
cp "${NEOLOGD_DIR}/COPYING" "${BUNDLE_DIR}/COPYING"

BUILT_AT_UTC="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
cat > "${BUNDLE_DIR}/metadata.json" <<EOF
{
  "neologd_repo": "${NEOLOGD_REPO}",
  "neologd_commit": "${NEOLOGD_SHA}",
  "seed_date": "${SEED_DATE}",
  "vibrato_ref": "${VIBRATO_REF}",
  "built_at_utc": "${BUILT_AT_UTC}",
  "dictionary_file": "system.dic.zst"
}
EOF

OUTPUT_DIR="${GITHUB_WORKSPACE:-$(pwd)}/dist"
mkdir -p "${OUTPUT_DIR}"
ASSET_PATH="${OUTPUT_DIR}/${ASSET_NAME}"
tar -C "${DIST_ROOT}" -cJf "${ASSET_PATH}" "${BUNDLE_DIR_NAME}"

RELEASE_TAG="neologd-${SEED_DATE}-${NEOLOGD_SHA7}"
RELEASE_TITLE="mecab-ipadic-neologd ${SEED_DATE} (${NEOLOGD_SHA7})"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "release_tag=${RELEASE_TAG}"
    echo "release_title=${RELEASE_TITLE}"
    echo "asset_path=${ASSET_PATH}"
    echo "seed_date=${SEED_DATE}"
    echo "neologd_sha=${NEOLOGD_SHA}"
    echo "vibrato_ref=${VIBRATO_REF}"
    echo "built_at_utc=${BUILT_AT_UTC}"
  } >> "${GITHUB_OUTPUT}"
fi

echo "[build] release tag: ${RELEASE_TAG}"
echo "[build] asset path: ${ASSET_PATH}"
