mod shared;
mod utils;
mod vendoring;
mod version;
mod elm_cli;

use anyhow::Result;
use dialoguer::Confirm;
use git2::{self, Repository};
use shared::*;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self};
use std::path::{Path, PathBuf};

pub struct CmdContext {
    pub yes: bool,
    pub root: PathBuf,
}

/// does the given file name match the elm-vendor file name?
fn is_elm_vendor_config_file_name(name: &OsStr) -> bool {
    name == "elm-vendor.json" || name == "elm-vendor.toml"
}

impl CmdContext {
    //region helpers
    /// See if the elm.json file for the repo is commited.
    pub(crate) fn check_if_elm_json_is_commited(&self) -> Result<bool> {
        // let repo = Repository::open(&self.root)?;
        let repo = Repository::open_ext::<_, OsString, _>(
            &self.root,
            git2::RepositoryOpenFlags::NO_SEARCH,
            std::iter::empty(),
        )?;
        let status = dbg!(repo.status_file(Path::new("elm.json"))?);

        Ok(status == git2::Status::CURRENT)
    }

    /// Try to find the elm-vendor file in the current directory.
    pub(crate) fn find_elm_vendor_json(&self) -> Result<Option<OsString>, anyhow::Error> {
        let mut elm_vendor_files = HashSet::new();
        for dir_ent_res in fs::read_dir(&self.root)? {
            let dir_ent = dir_ent_res?;
            let file_name = dir_ent.file_name();
            if is_elm_vendor_config_file_name(&file_name) {
                elm_vendor_files.insert(file_name);
            }
        }

        match elm_vendor_files.len() {
            1 => Ok(Some(elm_vendor_files.into_iter().next().unwrap())),
            0 => Ok(None),
            _ => anyhow::bail!("Multiple elm-vendor.{{json,toml}} found"),
        }
    }
    //endregion

    //region operations

    /// The init command.
    pub fn init(&self) -> Result<(), anyhow::Error> {
        if !self.yes {
            println!("{}", INIT_PROMPT);
            let ok = Confirm::new().with_prompt("Sound good?").interact()?;
            if !ok {
                return Ok(());
            }
        }

        if let Some(_) = self.find_elm_vendor_json()? {
            anyhow::bail!("An elm-vendor file already exists!");
        }

        let elm_json_file = fs::OpenOptions::new()
            .read(true)
            .open(self.root.join("elm.json"))?;
        let elm_json: NormalizedElmJson =
            serde_json::from_reader::<_, ElmJson>(elm_json_file)?.into();

        let elm_vendor = ElmVendor {
            main_deps: elm_json.dependencies,
            source_dirs: elm_json.source_dirs,
            vendored: Vec::new(),
            kind: elm_json.kind,
            extras: elm_json.other_fields,
        };

        let elm_vendor_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join("elm-vendor.json"))?;

        serde_json::to_writer_pretty(elm_vendor_file, &elm_vendor)?;

        // TODO explain where to go from here
        Ok(())
    }
    //endregion
}

const INIT_PROMPT: &'static str =
    "I'm going to extract all the user-set fields from elm.json, and add them to elm-vendor.json.";
