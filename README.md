# PostGit

The goal of PostGit is to integrate PostgreSQL schema diffing tools (e.g. `migra`) with Git to provide a modern development experience for PostgreSQL schemas.

This is a proof-of-concept, which started as my Hackathon Onboarding Project with [Commit](https://commit.dev/). Contributors are welcome.

## Concept

The goal of PostGit is to enable PostgreSQL schema developers to write clean, refactorable SQL code which does not rely on a list of ordered migration files and does not require developers to write idempotent scripts.

By leveraging a schema diffing tool, the `postgit push` command generates a migration script between two committed schemas and applies the migration to a target database.

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

### Configuration

The behaviour of PostGit can be configured through a combination of configuration file and command line arguments.

#### PostgreSQL config

PostGit relies on three databases:

- a diff engine source and a target where the respective schemas are deployed for the `diff` command. Those databases should only be used by PostGit as they are dropped and recreated every time
- a target database where the migrations from the `push` and `watch` commands are applied

A local `postgit.toml` file can be used to define the PostgreSQL connection parameters. The default configuration is equivalent to the following

```toml
[diff_engine]

[diff_engine.source]
dbname='postgit_diff_source'
host='localhost'
port=5432
user='postgres'

[diff_engine.target]
dbname='postgit_diff_target'
host='localhost'
port=5432
user='postgres'

[target]
dbname='postgres'
host='localhost'
port=5432
user='postgres'
```

PostGit supports the following [`libpq` environment variables](https://www.postgresql.org/docs/current/libpq-envars.html) for all three databases (the `postgit.toml` file takes precedence over env variables): `PGHOST`, `PGUSER`, `PGPORT`.

The `PGDATABASE` env variable can be used to specify the `target` database name.

#### Diff engine

PostGit relies on customisable external CLI tools to perform the schema diffing.

The current default schema diffing tool is [`migra`](https://github.com/djrobstep/migra), which can be installed by running `pip install migra psycopg2-binary`.

The diff tool can be configured with the `diff_engine.command` configuration option. The custom command must use two postgresql connection strings for the source and target databases as the positional arguments `$1` and `$2`, respectively.

For instance, to use migra, the `config.toml` would contain the following (the default behaviour is equivalent to this configuration):

```toml
[diff_engine]
command='migra --unsafe $1 $2'
```

Using the [CLI version of `pgAdmin4`](https://supabase.com/blog/supabase-cli#choosing-the-best-diff-tool) can be done with

```toml
[diff_engine]
command='docker run --network=host supabase/pgadmin-schema-diff $1 $2'
```

## SQL files management

As your database schema grows, you will most likely want to split your SQL code into multiple files.
To allow you to load multiple files in the desired order, PostGit supports a custom `-- import` syntax, e.g.:

`schema/schema.sql`

```sql
create schema my_app;
```

`schema/user.sql`

```sql
-- import schema/schema.sql

create table my_app.user (
  id int primary key generated always as identity,
  given_name text not null,
  family_name text,
  email text not null
);
```

**Important**:

- The paths specified in the import statements must be relative paths from the root of the repository.
- If you do not specify imports, all the files in the specified directory will be imported in lexicographical sorting order of their paths (i.e. in BFS order)
