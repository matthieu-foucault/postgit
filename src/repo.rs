use anyhow::{bail, Result};
use git_repository::bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::traverse::tree::Recorder;
use git_repository::{Commit, Repository};
use petgraph::algo::toposort;
use petgraph::prelude::{DiGraph, DiGraphMap, UnGraph};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

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

        let object_iter = recorder
            .records
            .iter()
            .filter(|entry| {
                matches!(entry.mode, EntryMode::Blob)
                    && entry.filepath.to_path().unwrap().starts_with(schema_path)
            })
            .map(|entry| repo.find_object(entry.oid))
            .filter_map(Result::ok);

        let mut script = String::new();
        for object in object_iter {
            script.push_str(object.data.to_str()?);
        }

        Ok(script)
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

fn merge_sql_scripts(sql_scripts: &HashMap<&str, &str>) -> Result<String> {
    let import_regex = Regex::new(r"(m?)^.*--\s*import\s+(.*)$").unwrap();
    let mut edges: Vec<(String, String)> = vec![];
    for (k, v) in sql_scripts.iter() {
        let mut parent = "".to_string();
        for path_part in k.split('/') {
            let mut full_file_path = path_part.to_string();
            if !parent.is_empty() {
                full_file_path = parent.clone() + "/" + path_part;
                edges.push((parent.clone(), full_file_path.clone()));
            }
            parent = full_file_path;
        }

        for group in import_regex.captures_iter(v) {
            edges.push((group[0].to_string(), k.to_string()));
        }
    }

    let str_edges: Vec<(&str, &str)> = edges
        .iter()
        .map(|&(ref x, ref y)| (x.as_str(), y.as_str()))
        .collect();

    let graph = DiGraphMap::<_, ()>::from_edges(str_edges);
    let sorted_nodes = toposort(&graph, None);
    match sorted_nodes {
        Ok(nodes) => Ok(nodes
            .iter()
            .map(|key| sql_scripts.get(key))
            .filter_map(|e| e)
            .map(|e| *e)
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
        r#"
    -- import schema/b

    create table foo.bar(
        id int primary key
    );
    "#,
    );
    scripts.insert(
        "schema/b",
        r#"
    create schema foo;
    "#,
    );

    let merged_script = merge_sql_scripts(&scripts);
    assert_eq!(
        r#"
    create schema foo;
    -- import schema/b

    create table foo.bar(
        id int primary key
    );
    "#
        .to_string(),
        merged_script.unwrap()
    );
}
