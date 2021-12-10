extern crate elm_vendor;

use anyhow::{Context, Error, Result};
use elm_vendor::*;
use fehler::throws;
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

//region tempfile abstraction
struct Tempy(Option<TempDir>);

impl Tempy {
    fn path_buf(&self) -> PathBuf {
        self.path().to_owned()
    }

    fn path(&self) -> &Path {
        self.as_ref().path()
    }
}

impl From<TempDir> for Tempy {
    fn from(t: TempDir) -> Self {
        Tempy(Some(t))
    }
}

impl Drop for Tempy {
    fn drop(&mut self) {
        if std::env::var("KEEP_DIRS").is_ok() {
            self.0.take().map(std::mem::forget);
        }
    }
}

impl AsRef<TempDir> for Tempy {
    fn as_ref(&self) -> &TempDir {
        self.0.as_ref().unwrap()
    }
}
//endregion

fn make_test_dir(file_type: &'static str) -> Result<Tempy> {
    let tempfile = tempfile::tempdir_in("test-data").context("Creating temp dir in test-data")?;
    fs::copy(
        format!("test-data/sample-elm-{}.json", file_type),
        tempfile.path().join("elm.json"),
    )
    .context("Copying sample to temp")?;
    Ok(tempfile.into())
}

fn make_repo(path: impl AsRef<Path>) -> Result<git2::Repository> {
    Ok(git2::Repository::init(path)?)
}

fn commit_elm_json(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    dbg!(repo.path());

    index.add_path(Path::new("elm.json"))?;
    index.write()?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let author = git2::Signature::new("foo", "bar", &git2::Time::new(0, 0))?;

    repo.commit(Some("HEAD"), &author, &author, "", &tree, &[])?;

    Ok(())
}

#[test]
#[throws]
fn test_init_pkg() {
    let tempdir = make_test_dir("package")?;
    let ctx = CmdContext {
        yes: true,
        root: tempdir.path_buf(),
    };
    ctx.init()?;
}

#[test]
#[throws]
fn test_init_app() {
    let tempdir = make_test_dir("app")?;
    let ctx = CmdContext {
        yes: true,
        root: tempdir.path_buf(),
    };
    ctx.init()?;
}

#[test]
#[throws]
fn test_vendor() {
    let tempdir = make_test_dir("app")?;
    dbg!(tempdir.path());
    let repo_path = pathdiff::diff_paths(tempdir.path(), std::env::current_dir()?).unwrap();
    let repo = make_repo(&repo_path)?;
    commit_elm_json(&repo)?;
    let ctx = CmdContext {
        yes: true,
        root: tempdir.path_buf(),
    };
    ctx.vendor()?;
}
