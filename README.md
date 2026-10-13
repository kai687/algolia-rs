# `algolia-rs`

An Algolia CLI built for CI pipelines.

One static binary, no interactive prompts, no config files, and output you
can act on without piping through `jq`. Every write waits until Algolia has
published it, so the next step in your pipeline sees the result.

There is an official [Algolia CLI](https://github.com/algolia/cli)
for interactive use. This project started as a Rust learning exercise and
now focuses on one job: keeping an Algolia index in sync from CI.

## Authentication

Set two environment variables:

```sh
export ALGOLIA_APPLICATION_ID=...
export ALGOLIA_API_KEY=...
```

The API key needs the `settings` ACL for `settings` commands, and
`addObject`, `deleteIndex`, and `settings` for `records replace`.

## Commands

### `settings diff`

Compare a local settings file with the live settings of an index.

```sh
algolia-rs settings diff INDEX --file settings.json
```

- Exit code `0` if they match, `1` if they differ, `2` on error.
- Every top-level setting is compared, in both directions.
  Settings only in the file and settings only in the index both count as drift.
- `--output json` prints `{"index": ..., "changes": [{"key", "local", "remote"}]}`.
- In GitHub Actions (`GITHUB_ACTIONS=true`), drift also produces an
  `::error::` annotation and a rendered diff in the job summary.

### `settings get`

Print the settings of an index as JSON. Use it to create `settings.json`.

```sh
algolia-rs settings get INDEX > settings.json
```

### `settings apply`

Apply a settings file to an index. Returns once the settings are live.

```sh
algolia-rs settings apply INDEX --file settings.json
```

### `records replace`

Replace all records of an index with records from a file or standard input.
Input is one JSON object per line. Every record needs an `objectID`.

```sh
my-crawler | algolia-rs records replace INDEX --file -
```

The command copies settings, rules, and synonyms to a temporary index,
uploads the records there, then moves the temporary index over the target.
The target index is never empty, and the temporary index is deleted on failure.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success (for `settings diff`: no drift) |
| 1 | `settings diff` found drift |
| 2 | Error (bad input, API error, missing credentials) |

## Example workflow

```yaml
- name: Check Algolia settings drift
  run: algolia-rs settings diff "$INDEX_NAME" --file settings.json

- name: Crawl and upload
  run: my-crawler | algolia-rs records replace "$INDEX_NAME" --file -

- name: Update index settings
  run: algolia-rs settings apply "$INDEX_NAME" --file settings.json
```

All steps need `ALGOLIA_APPLICATION_ID` and `ALGOLIA_API_KEY` in `env`.
