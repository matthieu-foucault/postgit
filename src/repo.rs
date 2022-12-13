use anyhow::{bail, Result};
use git_repository::bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::traverse::tree::Recorder;
use git_repository::{Commit, Object, Repository};
use petgraph::algo::toposort;
use petgraph::prelude::DiGraphMap;
use petgraph::Direction;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

pub fn get_schema_script(repo_path: &str, ref_or_sha1: &str, schema_path: &str) -> Result<String> {
    let repo_path = Path::new(repo_path);
    let mut schema_path = Path::new(schema_path);
    if let Ok(p) = schema_path.strip_prefix("./") {
        schema_path = p
    }

    let repo = git_repository::open(repo_path)?;
    let commit_option = try_find_commit(&repo, ref_or_sha1)?;

    if let Some(commit) = commit_option {
        let tree = commit.tree()?;

        let mut recorder = Recorder::default();

        tree.traverse().breadthfirst::<Recorder>(&mut recorder)?;

        let objects_with_path = recorder
            .records
            .iter()
            .filter(|entry| {
                matches!(entry.mode, EntryMode::Blob)
                    && entry.filepath.to_path().unwrap().starts_with(schema_path)
            })
            .filter_map(|entry| match repo.find_object(entry.oid) {
                Ok(object) => Some((entry.filepath.to_str().unwrap(), object)),
                Err(err) => {
                    eprintln!("Could not find object with id {}:/n{}", entry.oid, err);
                    None
                }
            })
            .collect::<Vec<(&str, Object)>>();

        if objects_with_path.len() == 1 {
            let script = objects_with_path[0].1.data.to_str()?.to_string();
            Ok(script)
        } else {
            let scripts = objects_with_path
                .iter()
                .map(|(path, object)| (*path, object.data.to_str().unwrap()))
                .collect::<HashMap<&str, &str>>();

            let script = merge_sql_scripts(&scripts)?;

            Ok(script)
        }
    } else {
        bail!("Didn't find source commit for ref {}", ref_or_sha1);
    }
}

fn try_find_commit<'repo>(
    repo: &'repo Repository,
    ref_or_sha1: &str,
) -> Result<Option<Commit<'repo>>> {
    let object_option = repo.rev_parse_single(ref_or_sha1)?.object();

    if let Ok(object) = object_option {
        let commit = object.try_into_commit()?;
        Ok(Some(commit))
    } else {
        Ok(None)
    }
}

/// Naive path normalization, removing occurences of `..` and `.` without supporting symlinks or checking whether files exists
/// The provided path must not start with `..`
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if component == Component::ParentDir {
            normalized.pop();
        } else if component != Component::CurDir {
            normalized.push(component);
        }
    }
    normalized
}

fn merge_sql_scripts(sql_scripts: &HashMap<&str, &str>) -> Result<String> {
    let import_regex = Regex::new(r"(?m)^.*--\s*import\s+(.*)$").unwrap();
    let mut graph = DiGraphMap::<&str, ()>::new();

    let mut edges: Vec<(String, String)> = vec![];
    for (k, v) in sql_scripts.iter() {
        graph.add_node(k);

        for group in import_regex.captures_iter(v) {
            let mut import_path = PathBuf::from(group[1].to_string());
            let first_component = import_path.components().next();
            if first_component == Some(std::path::Component::CurDir)
                || first_component == Some(std::path::Component::ParentDir)
            {
                import_path = PathBuf::from(k);
                import_path.pop();
                import_path.push(PathBuf::from(group[1].to_string()));
            }
            let normalized_path = normalize_path(import_path.as_path());
            edges.push((normalized_path.display().to_string(), k.to_string()));
        }
    }

    let str_edges: Vec<(&str, &str)> = edges
        .iter()
        .map(|(x, y)| (x.as_str(), y.as_str()))
        .collect();

    for e in str_edges {
        graph.add_edge(e.0, e.1, ());
    }

    let mut ordered_keys: Vec<&&str> = sql_scripts.keys().collect();
    ordered_keys.sort();

    // make sure every node has an edge, to have a deterministic topo sort
    for i in 0..ordered_keys.len() {
        if graph
            .neighbors_directed(ordered_keys[i], Direction::Incoming)
            .count()
            == 0
            && graph
                .neighbors_directed(ordered_keys[i], Direction::Outgoing)
                .count()
                == 0
        {
            if i == 0 {
                graph.add_edge(ordered_keys[0], ordered_keys[1], ());
            } else {
                graph.add_edge(ordered_keys[i - 1], ordered_keys[i], ());
            }
        }
    }

    let sorted_nodes = toposort(&graph, None);
    match sorted_nodes {
        Ok(nodes) => Ok(nodes
            .iter()
            .filter_map(|key| sql_scripts.get(key))
            .copied()
            .collect::<Vec<&str>>()
            .join("\n")),
        Err(_) => bail!("Dependency cycle found."),
    }
}

#[test]
fn it_merges_sql_scripts_in_order() {
    let mut scripts = HashMap::new();
    scripts.insert(
        "schema/a",
        r#"-- import schema/b

create table foo.bar(
    id int primary key
);
"#,
    );
    scripts.insert("schema/b", r#"create schema foo;"#);

    let merged_script = merge_sql_scripts(&scripts);
    assert_eq!(
        r#"create schema foo;
-- import schema/b

create table foo.bar(
    id int primary key
);
"#
        .to_string(),
        merged_script.unwrap()
    );
}

#[test]
fn it_merges_sql_scripts_in_bfs_order() {
    let mut scripts = HashMap::new();
    scripts.insert("a/b/c", "1");
    scripts.insert("a/b/d", "2");
    scripts.insert("a/e", "3");
    scripts.insert("a/f/g", "4");
    scripts.insert("a/f/h", "5");

    let merged_script = merge_sql_scripts(&scripts);
    assert_eq!("1\n2\n3\n4\n5".to_string(), merged_script.unwrap());
}

#[test]
fn it_merges_scripts_in_order_with_some_imports() {
    let mut scripts = HashMap::new();
    scripts.insert("a/b/c", "c");
    scripts.insert("a/b/d", "d");
    scripts.insert(
        "a/e",
        r#"-- import a/b/d
-- import a/f/h
e"#,
    );
    scripts.insert("a/f/g", "g");
    scripts.insert("a/f/h", "h");

    let merged_script = merge_sql_scripts(&scripts).unwrap();
    let lines: Vec<&str> = merged_script.lines().collect();

    assert!(
        lines.iter().position(|l| l.starts_with('d')).unwrap()
            < lines.iter().position(|l| l.starts_with('e')).unwrap()
    );
    assert!(
        lines.iter().position(|l| l.starts_with('h')).unwrap()
            < lines.iter().position(|l| l.starts_with('e')).unwrap()
    );
}

#[test]
fn it_imports_with_relative_paths() {
    let mut scripts = HashMap::new();
    scripts.insert("a", "a");
    scripts.insert("b/c", "c");
    scripts.insert(
        "b/d/e",
        r#"
-- import ../c
-- import ./f
e"#,
    );
    scripts.insert("b/d/f", "f");

    let merged_script = merge_sql_scripts(&scripts).unwrap();
    let lines: Vec<&str> = merged_script.lines().collect();

    assert!(
        lines.iter().position(|l| l.starts_with('c')).unwrap()
            < lines.iter().position(|l| l.starts_with('e')).unwrap()
    );
    assert!(
        lines.iter().position(|l| l.starts_with('f')).unwrap()
            < lines.iter().position(|l| l.starts_with('e')).unwrap()
    );
}
