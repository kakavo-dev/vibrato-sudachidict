# vibrato-ipadic-neologd

Build and release the latest `SudachiDict` (`full` edition) as a Vibrato system dictionary.

## Workflow

- Workflow file: `.github/workflows/release-neologd.yml`
- Trigger: `workflow_dispatch` only
- Permissions: `contents: write`

The workflow does the following:

1. Resolves the latest release of `WorksApplications/SudachiDict`.
2. Detects dictionary version from `sudachi-dictionary-<YYYYMMDD>-full.zip`.
3. Downloads raw dictionary sources from `sudachidict-raw`.
4. Concatenates `small_lex.csv`, `core_lex.csv`, and `notcore_lex.csv` into `lex.csv`.
5. Resolves `Sudachi` version from SudachiDict `build.gradle`.
6. Fetches `char.def` and `unk.def` from `WorksApplications/Sudachi` at the resolved version tag.
7. Compiles a Vibrato dictionary using `daac-tools/vibrato@v0.5.2`.
8. Runs a tokenize smoke test.
9. Packages `system.dic.zst`, `metadata.json`, `LICENSE-2.0.txt`, and `LEGAL` into one `tar.xz`.
10. Creates or updates the GitHub Release for the source-fixed tag.

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
- `built_at_utc`
- `dictionary_file`
