# PostGit

The goal of PostGit is to integrate PostgreSQL schema diffing tools (e.g. `migra`) with Git to provide a modern development experience for PostgreSQL schemas.

This is a proof-of-concept, which started as my Hackathon Onboarding Project with [Commit](https://commit.dev/). Contributors are welcome.

## Concept

The goal of PostGit is to enable PostgreSQL schema developers to write clean, refactorable SQL code which does not rely on a list of ordered migration files and does not require developers to write idempotent scripts.

By leveraging an schema diffing tool, the `postgit push` command generates a migration script between two committed schemas and applies the migration to a target database.

## Diff engine

PostGit relies on customisable external CLI tools to perform the schema diffing.

The current default schema diffing tool is [`migra`](https://github.com/djrobstep/migra), which can be installed by running `pip install migra psycopg2-binary`.

The diff tool can be configured with the `diff_engine.command` configuration option. The custom command must use two postgresql connection strings for the source and target databases as the positional arguments `$1` and `$2`, respectively.

For instance, to use migra, the `config.toml` would contain the following:

```toml
[diff_engine]
command='migra --unsafe $1 $2'
```

Using the CLI version of `pgAdmin4` can be done with

```toml
[diff_engine]
command='docker run supabase/pgadmin-schema-diff $1 $2'
```

## Usage

### Config file

A local `config.toml` file is required to tell PostGit how to connect to the database used for schema diffing, e.g.

```toml
[diff_engine]
command='migra --unsafe $1 $2'

[diff_engine.source]
dbname='postgres_vcs_source'
host='localhost'
port=5432
user='postgres'

[diff_engine.target]
dbname='postgres_vcs_target'
host='localhost'
port=5432
user='postgres'

[target]
dbname='postgit_test'
host='localhost'
port=5432
user='postgres'
```

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
