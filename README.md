# PostGit

POC - Use the power of Git for PostgreSQL schema migrations

## Prerequisites

`pip install migra psycopg2-binary`

## Usage

### Diff command

Prints the migration between two committed SQL files

`postgit diff [OPTIONS] --from <FROM> --to <TO> <PATH>`

Arguments:
`<PATH>` Path to the schema file

Options:

- `-r`, `--repo-path <REPO_PATH>` Path to the root of the git repository `[default: .]`

- `-f`, `--from <FROM>` Git commit where the source schema can be found
- `-t`, `--to <TO>` Git commit where the target schema can be found
- `--source-path <SOURCE_PATH>` Path to the source schema at the source ref, if different from the target path
