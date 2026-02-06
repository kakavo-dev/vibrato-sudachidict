# vibrato-sudachidict

Build and release the latest `SudachiDict` (`full` edition) as a Vibrato system dictionary.
Dictionary resources are converted by a Rust tool (`tools/sudachi-vibrato-converter`).

## Workflow

- Workflow file: `.github/workflows/release-sudachi.yml`
- Trigger:
  - `workflow_dispatch` (manual)
  - `schedule` daily (`0 0 * * *`, UTC)
- Permissions: `contents: write`

The workflow does the following:

1. Resolves the latest release of `WorksApplications/SudachiDict`.
2. Detects dictionary version from `sudachi-dictionary-<YYYYMMDD>-full.zip`.
3. Downloads raw dictionary sources from `sudachidict-raw`.
4. Concatenates `small_lex.csv`, `core_lex.csv`, and `notcore_lex.csv`.
5. Runs Rust converter tests.
6. Converts `lex.csv`, `unk.def`, and `char.def` with Rust.
7. Resolves `Sudachi` version from SudachiDict `build.gradle`.
8. Compiles a Vibrato dictionary using `daac-tools/vibrato@v0.5.2`.
9. Runs a tokenize smoke test.
10. Packages `system.dic.zst`, `metadata.json`, `LICENSE-2.0.txt`, and `LEGAL` into one `tar.xz`.
11. For scheduled runs, checks whether the latest SudachiDict tag is already released in this repository.
12. Creates or updates the GitHub Release only when needed.

## Automatic update detection

- GitHub Actions cannot directly subscribe to external repository release events.
- This repository polls `WorksApplications/SudachiDict` once per day.
- If the derived release tag already exists, the workflow exits via a skip job.
- Manual runs always build and upload with `--clobber`, even when the tag already exists.

## Compatibility policy

- Compatibility target: `jpreprocess`
- Compatibility mode: `safe-normalized`
- Feature schema: `mecab9-v1`
- Unknown or unsupported details are safely downgraded to `*`.
- Only MeCab-minimum fields are kept for lexicon features.

## Feature schema

`lex.csv` output keeps `surface,left_id,right_id,cost` and rewrites feature columns to exactly:

1. `pos1`
2. `pos2`
3. `pos3`
4. `pos4`
5. `ctype`
6. `cform`
7. `base`
8. `read`
9. `pron`

Source mapping:

- `pos1..4`: normalized from Sudachi `col5..8`
- `ctype`: normalized from Sudachi `col9`
- `cform`: normalized from Sudachi `col10`
- `base`: Sudachi `col4` (empty => `*`)
- `read`: Sudachi `col11` (empty => `*`)
- `pron`: same as `read`

Sudachi columns after `col12` are dropped.

## unk.def schema

`unk.def` is also converted to MeCab-minimum fields:

`category,left_id,right_id,cost,pos1,pos2,pos3,pos4,ctype,cform,base,read,pron`

- POS/CType/CForm normalization is the same as `lex.csv`.
- `base/read/pron` are fixed to `*`.

## char.def conversion

- Keep comments and blank lines.
- Strip `NOOOVBOW` from codepoint-range lines.
- Drop range lines that become category-less after stripping.

## Latest resolution policy

- `latest SudachiDict release` is the source of truth.
- Edition is fixed to `full`.
- Dictionary version is parsed from the release asset name.
- `char.def`/`unk.def` are selected from the Sudachi version referenced by that SudachiDict release.

## Release naming

- Tag: `sudachi-<DICT_VERSION>-full-vibrato-v0_5_2`
- Asset: `sudachidict-<DICT_VERSION>-full+vibrato-v0_5_2.tar.xz`

If the same tag already exists, the workflow updates that release and replaces the asset with `--clobber`.

## Metadata

`metadata.json` includes:

- `sudachidict_repo`
- `sudachidict_release_tag`
- `sudachidict_dict_version`
- `edition`
- `sudachi_repo`
- `sudachi_version`
- `sudachi_tag`
- `vibrato_ref`
- `compat_target`
- `compat_mode`
- `feature_schema`
- `normalized_pos_rows`
- `fallback_ctype_rows`
- `fallback_cform_rows`
- `built_at_utc`
- `dictionary_file`
