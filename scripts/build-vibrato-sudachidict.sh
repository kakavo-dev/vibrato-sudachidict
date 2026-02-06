#!/usr/bin/env bash
set -euo pipefail

SUDACHIDICT_REPO="WorksApplications/SudachiDict"
SUDACHI_REPO="WorksApplications/Sudachi"
VIBRATO_REPO="https://github.com/daac-tools/vibrato.git"
VIBRATO_REF="v0.5.2"
EDITION="full"
RAW_BASE_URL="https://d2ej7fkh96fzlu.cloudfront.net/sudachidict-raw"

WORK_BASE="$(mktemp -d "${RUNNER_TEMP:-/tmp}/vibrato-sudachidict.XXXXXX")"
RAW_DIR="${WORK_BASE}/raw"
UNZIP_DIR="${WORK_BASE}/unzipped"
BUILD_DIR="${WORK_BASE}/build"
DIST_ROOT="${WORK_BASE}/dist"
VIBRATO_DIR="${WORK_BASE}/vibrato"

mkdir -p "${RAW_DIR}" "${UNZIP_DIR}" "${BUILD_DIR}" "${DIST_ROOT}"

download_with_retry() {
  local url="$1"
  local out="$2"
  local attempt

  for attempt in 1 2 3; do
    if curl --fail --location --silent --show-error "${url}" --output "${out}"; then
      return 0
    fi
    if [[ "${attempt}" -lt 3 ]]; then
      sleep $((attempt * 2))
    fi
  done
  return 1
}

decode_repo_file() {
  local repo="$1"
  local path="$2"
  local ref="$3"
  local out="$4"

  gh api "repos/${repo}/contents/${path}?ref=${ref}" --jq '.content' \
    | tr -d '\n' \
    | base64 --decode \
    > "${out}"
}

sanitize_char_def_for_vibrato() {
  local in_path="$1"
  local out_path="$2"

  awk '
  {
    line = $0
    sub(/\r$/, "", line)

    if (line ~ /^[[:space:]]*$/ || line ~ /^[[:space:]]*#/) {
      print line
      next
    }

    if (line ~ /^[[:space:]]*0x[0-9A-Fa-f]+(\.\.0x[0-9A-Fa-f]+)?[[:space:]]+/) {
      comment = ""
      body = line
      cidx = index(body, "#")
      if (cidx > 0) {
        comment = substr(body, cidx)
        body = substr(body, 1, cidx - 1)
      }

      token_count = split(body, raw, /[[:space:]]+/)
      n = 0
      for (i = 1; i <= token_count; i++) {
        if (raw[i] != "") {
          n++
          tok[n] = raw[i]
        }
      }

      if (n >= 2) {
        out = tok[1]
        kept = 0
        for (i = 2; i <= n; i++) {
          if (tok[i] == "NOOOVBOW") {
            continue
          }
          out = out " " tok[i]
          kept++
        }

        # If a line becomes category-less after stripping NOOOVBOW, skip it
        # and keep the category from previous range lines.
        if (kept == 0) {
          next
        }

        if (comment != "") {
          out = out " " comment
        }
        print out
        next
      }
    }

    print line
  }' "${in_path}" > "${out_path}"
}

echo "[build] resolve latest SudachiDict release"
SUDACHIDICT_RELEASE_TAG="$(gh api "repos/${SUDACHIDICT_REPO}/releases/latest" --jq '.tag_name')"
FULL_ASSET_NAME="$(
  gh api "repos/${SUDACHIDICT_REPO}/releases/latest" \
    --jq '.assets[] | select(.name | test("^sudachi-dictionary-[0-9]{8}-full\\.zip$")) | .name' \
    | head -n 1
)"

if [[ -z "${FULL_ASSET_NAME}" ]]; then
  echo "[error] full edition asset not found in latest release." >&2
  exit 1
fi

DICT_VERSION="$(echo "${FULL_ASSET_NAME}" | sed -E 's/^sudachi-dictionary-([0-9]{8})-full\.zip$/\1/')"
if [[ ! "${DICT_VERSION}" =~ ^[0-9]{8}$ ]]; then
  echo "[error] failed to parse dict version from asset name: ${FULL_ASSET_NAME}" >&2
  exit 1
fi

BUILD_GRADLE_PATH="${WORK_BASE}/build.gradle"
decode_repo_file "${SUDACHIDICT_REPO}" "build.gradle" "${SUDACHIDICT_RELEASE_TAG}" "${BUILD_GRADLE_PATH}"

SUDACHI_VERSION="$(
  sed -nE "s/.*com\\.worksap\\.nlp:sudachi:([0-9]+\\.[0-9]+\\.[0-9]+).*/\\1/p" "${BUILD_GRADLE_PATH}" \
    | head -n 1
)"
if [[ -z "${SUDACHI_VERSION}" ]]; then
  echo "[error] failed to parse Sudachi version from build.gradle" >&2
  exit 1
fi
SUDACHI_TAG="v${SUDACHI_VERSION}"

echo "[build] download raw SudachiDict resources: ${DICT_VERSION}"
download_with_retry "${RAW_BASE_URL}/${DICT_VERSION}/small_lex.zip" "${RAW_DIR}/small_lex.zip"
download_with_retry "${RAW_BASE_URL}/${DICT_VERSION}/core_lex.zip" "${RAW_DIR}/core_lex.zip"
download_with_retry "${RAW_BASE_URL}/${DICT_VERSION}/notcore_lex.zip" "${RAW_DIR}/notcore_lex.zip"
download_with_retry "${RAW_BASE_URL}/matrix.def.zip" "${RAW_DIR}/matrix.def.zip"

echo "[build] extract raw resources"
unzip -q "${RAW_DIR}/small_lex.zip" -d "${UNZIP_DIR}/small"
unzip -q "${RAW_DIR}/core_lex.zip" -d "${UNZIP_DIR}/core"
unzip -q "${RAW_DIR}/notcore_lex.zip" -d "${UNZIP_DIR}/notcore"
unzip -q "${RAW_DIR}/matrix.def.zip" -d "${UNZIP_DIR}/matrix"

SMALL_CSV="$(find "${UNZIP_DIR}/small" -type f -name 'small_lex.csv' | head -n 1)"
CORE_CSV="$(find "${UNZIP_DIR}/core" -type f -name 'core_lex.csv' | head -n 1)"
NOTCORE_CSV="$(find "${UNZIP_DIR}/notcore" -type f -name 'notcore_lex.csv' | head -n 1)"
MATRIX_DEF="$(find "${UNZIP_DIR}/matrix" -type f -name 'matrix.def' | head -n 1)"

if [[ -z "${SMALL_CSV}" || -z "${CORE_CSV}" || -z "${NOTCORE_CSV}" || -z "${MATRIX_DEF}" ]]; then
  echo "[error] one or more required raw dictionary files are missing after unzip." >&2
  exit 1
fi

LEXICON_RAW_PATH="${BUILD_DIR}/lex.raw.csv"
LEXICON_PATH="${BUILD_DIR}/lex.csv"
cat "${SMALL_CSV}" "${CORE_CSV}" "${NOTCORE_CSV}" > "${LEXICON_RAW_PATH}"

# Sudachi lexicon can include split-only entries with negative connection ids.
# Vibrato's compile expects non-negative u16 ids, so those rows are skipped.
ruby -rcsv -e '
input_path, output_path = ARGV
skipped = 0
written = 0
CSV.open(output_path, "w", row_sep: "\n", force_quotes: false) do |w|
  CSV.foreach(input_path, encoding: "UTF-8") do |row|
    next if row.nil? || row.empty?
    if row.length < 4
      raise "invalid lex row (too few columns): #{row.inspect}"
    end
    left = Integer(row[1], 10)
    right = Integer(row[2], 10)
    # row[3] should be parseable as cost even if we do not use it here.
    Integer(row[3], 10)
    if left < 0 || right < 0
      skipped += 1
      next
    end
    w << row
    written += 1
  end
end
warn "[build] lex rows: written=#{written}, skipped_negative_conn_ids=#{skipped}"
' "${LEXICON_RAW_PATH}" "${LEXICON_PATH}"

CHAR_DEF="${BUILD_DIR}/char.def"
CHAR_DEF_RAW="${BUILD_DIR}/char.raw.def"
UNK_DEF="${BUILD_DIR}/unk.def"
decode_repo_file "${SUDACHI_REPO}" "src/main/resources/char.def" "${SUDACHI_TAG}" "${CHAR_DEF_RAW}"
sanitize_char_def_for_vibrato "${CHAR_DEF_RAW}" "${CHAR_DEF}"
decode_repo_file "${SUDACHI_REPO}" "src/main/resources/unk.def" "${SUDACHI_TAG}" "${UNK_DEF}"

SUDACHIDICT_LICENSE="${BUILD_DIR}/LICENSE-2.0.txt"
SUDACHIDICT_LEGAL="${BUILD_DIR}/LEGAL"
decode_repo_file "${SUDACHIDICT_REPO}" "LICENSE-2.0.txt" "${SUDACHIDICT_RELEASE_TAG}" "${SUDACHIDICT_LICENSE}"
decode_repo_file "${SUDACHIDICT_REPO}" "LEGAL" "${SUDACHIDICT_RELEASE_TAG}" "${SUDACHIDICT_LEGAL}"

echo "[build] clone vibrato: ${VIBRATO_REF}"
git clone --depth 1 --branch "${VIBRATO_REF}" "${VIBRATO_REPO}" "${VIBRATO_DIR}"

SYSTEM_DIC_PATH="${BUILD_DIR}/system.dic.zst"
echo "[build] compile Vibrato dictionary"
cargo run --release --manifest-path "${VIBRATO_DIR}/Cargo.toml" -p compile -- \
  -l "${LEXICON_PATH}" \
  -m "${MATRIX_DEF}" \
  -u "${UNK_DEF}" \
  -c "${CHAR_DEF}" \
  -o "${SYSTEM_DIC_PATH}"

echo "[build] smoke test"
printf '%s\n' "東京都に行く" \
  | cargo run --release --manifest-path "${VIBRATO_DIR}/Cargo.toml" -p tokenize -- \
      -i "${SYSTEM_DIC_PATH}" >/dev/null

ASSET_NAME="sudachidict-${DICT_VERSION}-${EDITION}+vibrato-v0_5_2.tar.xz"
BUNDLE_DIR_NAME="sudachidict-${DICT_VERSION}-${EDITION}+vibrato-v0_5_2"
BUNDLE_DIR="${DIST_ROOT}/${BUNDLE_DIR_NAME}"
mkdir -p "${BUNDLE_DIR}"

cp "${SYSTEM_DIC_PATH}" "${BUNDLE_DIR}/system.dic.zst"
cp "${SUDACHIDICT_LICENSE}" "${BUNDLE_DIR}/LICENSE-2.0.txt"
cp "${SUDACHIDICT_LEGAL}" "${BUNDLE_DIR}/LEGAL"

BUILT_AT_UTC="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
cat > "${BUNDLE_DIR}/metadata.json" <<EOF
{
  "sudachidict_repo": "${SUDACHIDICT_REPO}",
  "sudachidict_release_tag": "${SUDACHIDICT_RELEASE_TAG}",
  "sudachidict_dict_version": "${DICT_VERSION}",
  "edition": "${EDITION}",
  "sudachi_repo": "${SUDACHI_REPO}",
  "sudachi_version": "${SUDACHI_VERSION}",
  "sudachi_tag": "${SUDACHI_TAG}",
  "vibrato_ref": "${VIBRATO_REF}",
  "built_at_utc": "${BUILT_AT_UTC}",
  "dictionary_file": "system.dic.zst"
}
EOF

OUTPUT_DIR="${GITHUB_WORKSPACE:-$(pwd)}/dist"
mkdir -p "${OUTPUT_DIR}"
ASSET_PATH="${OUTPUT_DIR}/${ASSET_NAME}"
tar -C "${DIST_ROOT}" -cJf "${ASSET_PATH}" "${BUNDLE_DIR_NAME}"

RELEASE_TAG="sudachi-${DICT_VERSION}-${EDITION}-vibrato-v0_5_2"
RELEASE_TITLE="SudachiDict ${DICT_VERSION} ${EDITION} (Vibrato ${VIBRATO_REF})"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "release_tag=${RELEASE_TAG}"
    echo "release_title=${RELEASE_TITLE}"
    echo "asset_path=${ASSET_PATH}"
    echo "dict_version=${DICT_VERSION}"
    echo "edition=${EDITION}"
    echo "sudachidict_release_tag=${SUDACHIDICT_RELEASE_TAG}"
    echo "sudachi_version=${SUDACHI_VERSION}"
    echo "vibrato_ref=${VIBRATO_REF}"
    echo "built_at_utc=${BUILT_AT_UTC}"
  } >> "${GITHUB_OUTPUT}"
fi

echo "[build] release tag: ${RELEASE_TAG}"
echo "[build] asset path: ${ASSET_PATH}"
