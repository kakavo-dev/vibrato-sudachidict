#!/usr/bin/env bash
set -euo pipefail

SUDACHIDICT_REPO="WorksApplications/SudachiDict"
SUDACHI_REPO="WorksApplications/Sudachi"
VIBRATO_REPO="https://github.com/daac-tools/vibrato.git"
VIBRATO_REF="v0.5.2"
EDITION="full"
RAW_BASE_URL="https://d2ej7fkh96fzlu.cloudfront.net/sudachidict-raw"
COMPAT_TARGET="jpreprocess"
COMPAT_MODE="safe-normalized"
FEATURE_SCHEMA="surface+ipadic9-v2"

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
NORM_STATS_PATH="${BUILD_DIR}/normalization_stats.env"
cat "${SMALL_CSV}" "${CORE_CSV}" "${NOTCORE_CSV}" > "${LEXICON_RAW_PATH}"

# Sudachi lexicon can include split-only entries with negative connection ids.
# Vibrato's compile expects non-negative u16 ids, so those rows are skipped.
# Details columns are normalized for jpreprocess compatibility and feature
# columns are rebuilt as:
# surface,pos1,pos2,pos3,pos4,ctype,cform,base,read,pron
ruby -rcsv -e '
input_path, output_path, stats_path = ARGV
skipped_negative_conn_ids = 0
written = 0
normalized_pos_rows = 0
fallback_ctype_rows = 0
fallback_cform_rows = 0

ALLOWED_CTYPE = [
  "*",
  "ラ変",
  "不変化型",
  "カ変・クル",
  "カ変・来ル",
  "サ変・スル",
  "サ変・−スル",
  "サ変・−ズル",
  "一段",
  "一段・病メル",
  "一段・クレル",
  "一段・得ル",
  "一段・ル",
  "下二・ア行",
  "下二・カ行",
  "下二・ガ行",
  "下二・サ行",
  "下二・ザ行",
  "下二・タ行",
  "下二・ダ行",
  "下二・ナ行",
  "下二・ハ行",
  "下二・バ行",
  "下二・マ行",
  "下二・ヤ行",
  "下二・ラ行",
  "下二・ワ行",
  "下二・得",
  "形容詞・アウオ段",
  "形容詞・イ段",
  "形容詞・イイ",
  "五段・カ行イ音便",
  "五段・カ行促音便",
  "五段・カ行促音便ユク",
  "五段・ガ行",
  "五段・サ行",
  "五段・タ行",
  "五段・ナ行",
  "五段・バ行",
  "五段・マ行",
  "五段・ラ行",
  "五段・ラ行アル",
  "五段・ラ行特殊",
  "五段・ワ行ウ音便",
  "五段・ワ行促音便",
  "四段・カ行",
  "四段・ガ行",
  "四段・サ行",
  "四段・タ行",
  "四段・バ行",
  "四段・マ行",
  "四段・ラ行",
  "四段・ハ行",
  "上二・ダ行",
  "上二・ハ行",
  "特殊・ナイ",
  "特殊・タイ",
  "特殊・タ",
  "特殊・ダ",
  "特殊・デス",
  "特殊・ドス",
  "特殊・ジャ",
  "特殊・マス",
  "特殊・ヌ",
  "特殊・ヤ",
  "文語・ベシ",
  "文語・ゴトシ",
  "文語・ナリ",
  "文語・マジ",
  "文語・シム",
  "文語・キ",
  "文語・ケリ",
  "文語・ル",
  "文語・リ",
].to_h { |v| [v, true] }.freeze

ALLOWED_CFORM = [
  "*",
  "ガル接続",
  "音便基本形",
  "仮定形",
  "仮定縮約１",
  "仮定縮約２",
  "基本形",
  "基本形-促音便",
  "現代基本形",
  "体言接続",
  "体言接続特殊",
  "体言接続特殊２",
  "文語基本形",
  "未然ウ接続",
  "未然ヌ接続",
  "未然レル接続",
  "未然形",
  "未然特殊",
  "命令ｅ",
  "命令ｉ",
  "命令ｒｏ",
  "命令ｙｏ",
  "連用ゴザイ接続",
  "連用タ接続",
  "連用テ接続",
  "連用デ接続",
  "連用ニ接続",
  "連用形",
].to_h { |v| [v, true] }.freeze

def normalize_pos(pos0)
  case pos0
  when "名詞", "代名詞", "形状詞", "接尾辞"
    ["名詞", "一般", "*", "*"]
  when "動詞"
    ["動詞", "自立", "*", "*"]
  when "形容詞"
    ["形容詞", "自立", "*", "*"]
  when "助詞"
    ["助詞", "格助詞", "一般", "*"]
  when "助動詞"
    ["助動詞", "*", "*", "*"]
  when "副詞"
    ["副詞", "一般", "*", "*"]
  when "接続詞"
    ["接続詞", "*", "*", "*"]
  when "連体詞"
    ["連体詞", "*", "*", "*"]
  when "感動詞"
    ["感動詞", "*", "*", "*"]
  when "接頭辞", "接頭詞"
    ["接頭詞", "名詞接続", "*", "*"]
  when "記号", "補助記号", "空白"
    ["記号", "一般", "*", "*"]
  when "フィラー"
    ["フィラー", "*", "*", "*"]
  else
    ["その他", "*", "*", "*"]
  end
end

def normalize_ctype(value)
  src = value.to_s.strip
  src = "*" if src.empty?
  canonical = src.gsub(/[[:space:]]+/, "")
  canonical = canonical.tr("　", "")
  canonical = canonical.gsub(/[\-－−]/, "・")

  canonical = "五段・ワ行ウ音便" if canonical == "五段・ワア行"
  canonical = canonical.sub(/^サ変・スル$/, "サ変・−スル")
  canonical = canonical.sub(/^サ変・ズル$/, "サ変・−ズル")
  canonical = canonical.sub(/^サ変・ｰスル$/, "サ変・−スル")
  canonical = canonical.sub(/^サ変・ｰズル$/, "サ変・−ズル")
  canonical = canonical.sub(/^サ変・ースル$/, "サ変・−スル")
  canonical = canonical.sub(/^サ変・ーズル$/, "サ変・−ズル")
  canonical = canonical.sub(/^サ変・・スル$/, "サ変・−スル")
  canonical = canonical.sub(/^サ変・・ズル$/, "サ変・−ズル")

  if ALLOWED_CTYPE.key?(canonical)
    [canonical, false]
  else
    fallback = src != "*"
    ["*", fallback]
  end
end

def normalize_cform(value)
  src = value.to_s.strip
  src = "*" if src.empty?
  canonical = src.gsub(/[[:space:]]+/, "")
  canonical = canonical.tr("　", "")

  canonical =
    case canonical
    when /\A終止形.*\z/, /\A連体形.*\z/, "終止連体形"
      "基本形"
    when /\A連用形.*\z/
      "連用形"
    when /\A未然形.*\z/
      "未然形"
    when /\A仮定形.*\z/
      "仮定形"
    when /\A命令形.*\z/
      "命令ｙｏ"
    when "意志推量形"
      "未然ウ接続"
    else
      canonical
    end

  if ALLOWED_CFORM.key?(canonical)
    [canonical, false]
  else
    fallback = src != "*"
    ["*", fallback]
  end
end

CSV.open(output_path, "w", row_sep: "\n", force_quotes: false) do |w|
  CSV.foreach(input_path, encoding: "UTF-8") do |row|
    next if row.nil? || row.empty?
    if row.length < 11
      raise "invalid lex row (too few columns): #{row.inspect}"
    end
    original = row.dup
    left = Integer(row[1], 10)
    right = Integer(row[2], 10)
    # row[3] should be parseable as cost even if we do not use it here.
    Integer(row[3], 10)
    if left < 0 || right < 0
      skipped_negative_conn_ids += 1
      next
    end

    original_pos = original.values_at(5, 6, 7, 8).map do |value|
      text = value.to_s.strip
      text.empty? ? "*" : text
    end
    normalized_pos = normalize_pos(original[5].to_s.strip)
    if original_pos != normalized_pos
      normalized_pos_rows += 1
    end

    ctype, ctype_fallback = normalize_ctype(original[9])
    fallback_ctype_rows += 1 if ctype_fallback

    cform, cform_fallback = normalize_cform(original[10])
    fallback_cform_rows += 1 if cform_fallback

    base = original[4].to_s.strip
    base = "*" if base.empty?

    read = original[11].to_s.strip
    read = "*" if read.empty?
    pron = read

    extra = original.length > 12 ? original[12..] : []
    output_row = [
      original[0], original[1], original[2], original[3],
      original[0],
      normalized_pos[0], normalized_pos[1], normalized_pos[2], normalized_pos[3],
      ctype, cform, base, read, pron,
      *extra
    ]

    w << output_row
    written += 1
  end
end

File.write(
  stats_path,
  [
    "written=#{written}",
    "skipped_negative_conn_ids=#{skipped_negative_conn_ids}",
    "normalized_pos_rows=#{normalized_pos_rows}",
    "fallback_ctype_rows=#{fallback_ctype_rows}",
    "fallback_cform_rows=#{fallback_cform_rows}",
  ].join("\n") + "\n",
)
warn "[build] lex rows: written=#{written}, skipped_negative_conn_ids=#{skipped_negative_conn_ids}, normalized_pos_rows=#{normalized_pos_rows}, fallback_ctype_rows=#{fallback_ctype_rows}, fallback_cform_rows=#{fallback_cform_rows}"
' "${LEXICON_RAW_PATH}" "${LEXICON_PATH}" "${NORM_STATS_PATH}"
source "${NORM_STATS_PATH}"

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
  "compat_target": "${COMPAT_TARGET}",
  "compat_mode": "${COMPAT_MODE}",
  "feature_schema": "${FEATURE_SCHEMA}",
  "normalized_pos_rows": ${normalized_pos_rows},
  "fallback_ctype_rows": ${fallback_ctype_rows},
  "fallback_cform_rows": ${fallback_cform_rows},
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
    echo "compat_target=${COMPAT_TARGET}"
    echo "compat_mode=${COMPAT_MODE}"
    echo "feature_schema=${FEATURE_SCHEMA}"
    echo "normalized_pos_rows=${normalized_pos_rows}"
    echo "fallback_ctype_rows=${fallback_ctype_rows}"
    echo "fallback_cform_rows=${fallback_cform_rows}"
    echo "built_at_utc=${BUILT_AT_UTC}"
  } >> "${GITHUB_OUTPUT}"
fi

echo "[build] release tag: ${RELEASE_TAG}"
echo "[build] asset path: ${ASSET_PATH}"
