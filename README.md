# vibrato-ipadic-neologd

Build and release the latest `mecab-ipadic-neologd` as a Vibrato system dictionary.

## Workflow

- Workflow file: `.github/workflows/release-neologd.yml`
- Trigger: `workflow_dispatch` only
- Permissions: `contents: write`

The workflow does the following:

1. Builds the latest `neologd/mecab-ipadic-neologd` from `master`.
2. Concatenates generated `*.csv` lexicon files into `lex.csv`.
3. Compiles a Vibrato dictionary using `daac-tools/vibrato@v0.5.2`.
4. Runs a tokenize smoke test.
5. Packages `system.dic.zst`, `metadata.json`, and `COPYING` into one `tar.xz`.
6. Creates or updates the GitHub Release for the source-fixed tag.

## Release naming

- Tag: `neologd-<SEED_DATE>-<NEOLOGD_SHA7>`
- Asset: `ipadic-neologd-<SEED_DATE>+n<NEOLOGD_SHA7>+vibrato-v0_5_2.tar.xz`

If the same tag already exists, the workflow updates that release and replaces the asset with `--clobber`.

## Metadata

`metadata.json` includes:

- `neologd_repo`
- `neologd_commit`
- `seed_date`
- `vibrato_ref`
- `built_at_utc`
- `dictionary_file`
