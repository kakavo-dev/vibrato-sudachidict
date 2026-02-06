# vibrato-ipadic-neologd

Build and release the latest `SudachiDict` (`full` edition) as a Vibrato system dictionary.
The generated dictionary details are normalized for `jpreprocess` compatibility.

## Workflow

- Workflow file: `.github/workflows/release-neologd.yml`
- Trigger: `workflow_dispatch` only
- Permissions: `contents: write`

The workflow does the following:

1. Resolves the latest release of `WorksApplications/SudachiDict`.
2. Detects dictionary version from `sudachi-dictionary-<YYYYMMDD>-full.zip`.
3. Downloads raw dictionary sources from `sudachidict-raw`.
4. Concatenates `small_lex.csv`, `core_lex.csv`, and `notcore_lex.csv`.
5. Rebuilds feature columns as `surface + IPADIC-compatible 9 fields` and normalizes `POS`/`CType`/`CForm` for `jpreprocess`.
6. Resolves `Sudachi` version from SudachiDict `build.gradle`.
7. Fetches `char.def` and `unk.def` from `WorksApplications/Sudachi` at the resolved version tag.
8. Compiles a Vibrato dictionary using `daac-tools/vibrato@v0.5.2`.
9. Runs a tokenize smoke test.
10. Packages `system.dic.zst`, `metadata.json`, `LICENSE-2.0.txt`, and `LEGAL` into one `tar.xz`.
11. Creates or updates the GitHub Release for the source-fixed tag.

## Compatibility policy

- Compatibility target: `jpreprocess`
- Compatibility mode: `safe-normalized`
- Feature schema: `surface+ipadic9-v2`
- Unknown or unsupported details are safely downgraded to `*` (or fixed safe POS tuples).
- As a tradeoff, original Sudachi detail granularity is partially simplified.

## Feature schema

`lex.csv` output keeps `surface,left_id,right_id,cost` as-is, then rebuilds feature slots as:

1. `surface` (same as lexical surface; downstream strips this first slot)
2. `pos1` (from Sudachi `col5`, normalized)
3. `pos2` (from Sudachi `col6`, normalized tuple output)
4. `pos3` (from Sudachi `col7`, normalized tuple output)
5. `pos4` (from Sudachi `col8`, normalized tuple output)
6. `ctype` (from Sudachi `col9`, normalized)
7. `cform` (from Sudachi `col10`, normalized)
8. `base` (from Sudachi `col4`)
9. `read` (from Sudachi `col11`, empty => `*`)
10. `pron` (same as `read`, empty => `*`)

Sudachi source details from `col12` and later are appended after `pron` in fixed order.

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
