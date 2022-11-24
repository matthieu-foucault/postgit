use std::path::Path;

use anyhow::{bail, Result};
use git_repository::bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::traverse::tree::Recorder;
use git_repository::{Commit, Repository};

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
