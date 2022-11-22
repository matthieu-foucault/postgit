# PostGit

POC - Use the power of Git for PostgreSQL schema migrations

## Prerequisites

`pip install migra psycopg2-binary`

## Usage

### Diff command

Prints the migration between two committed SQL files

`postgit diff [OPTIONS] --from <FROM> --to <TO> <PATH>`

Arguments:
`<PATH>` Path to the schema file or directory, relative to the repo root

Options:

- `-r`, `--repo-path <REPO_PATH>` Path to the root of the git repository `[default: .]`

- `-f`, `--from <FROM>` Git commit where the source schema can be found
- `-t`, `--to <TO>` Git commit where the target schema can be found
- `--source-path <SOURCE_PATH>` Path to the source schema at the source ref, if different from the target path

### Push command

Applies the migration between two committed SQL files onto the target database

`postgit push [OPTIONS] --from <FROM> --to <TO> <PATH>`

Arguments:
`<PATH>` Path to the schema file or directory, relative to the repo root

Options:

- `-r`, `--repo-path <REPO_PATH>` Path to the root of the git repository `[default: .]`

- `-f`, `--from <FROM>` Git commit where the source schema can be found
- `-t`, `--to <TO>` Git commit where the target schema can be found
- `--source-path <SOURCE_PATH>` Path to the source schema at the source ref, if different from the target path

### Watch command

Watches a directory and applies the migrations to the target database

Usage: `postgit watch <PATH>`

Arguments:
`<PATH>` Path to the directory to watch
