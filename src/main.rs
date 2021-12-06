mod utils;

use anyhow::Context;
use dialoguer::Confirm;
use git2::{self, Repository};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::string::ToString;
use thiserror::Error;
use utils::*;

// region configs
#[derive(Deserialize, Serialize)]
struct ElmPackage {
    #[serde(rename = "source-directories")]
    source_dirs: Vec<PathBuf>,
    dependencies: HashMap<String, String>,
    #[serde(flatten)]
    other_fields: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct ElmAppDeps {
    direct: HashMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
enum ElmJson {
    #[serde(rename = "application")]
    Application(ElmApplication),
    #[serde(rename = "package")]
    Package(ElmPackage),
}

impl ElmJson {
    fn kind(&self) -> ElmJsonKind {
        match self {
            ElmJson::Application(_) => ElmJsonKind::Application,
            ElmJson::Package(_) => ElmJsonKind::Package
        }
    }
}

#[derive(Serialize, Deserialize)]
enum ElmJsonKind {
    Application,
    Package,
}

#[derive(Deserialize, Serialize)]
struct ElmApplication {
    #[serde(rename = "source-directories")]
    source_dirs: Vec<PathBuf>,
    dependencies: ElmAppDeps,
    #[serde(flatten)]
    other_fields: HashMap<String, serde_json::Value>,
}

impl ElmPackage {
    fn is_lamdera_project(&self) -> bool {
        unimplemented!()
    }
}

/// The elm-vendor.{json,toml} config file
#[derive(Deserialize, Serialize)]
struct ElmVendor {
    #[serde(rename = "main-deps")]
    main_deps: HashMap<String, String>,
    vendored: Vec<String>,
    #[serde(rename = "type")]
    kind: ElmJsonKind,
    extras: HashMap<String, serde_json::Value>,
}
// endregion

//region error handling
#[derive(Debug)]
enum SerdeError {
    Toml(toml::de::Error),
    Json(serde_json::Error),
}

trait WithDepName: Sized {
    fn with_name_conv(self, name: impl ToString) -> VendorChangeError {
        self.with_name(name.to_string())
    }
    fn with_name(self, name: String) -> VendorChangeError;
}

trait WithDepNameExt<T> {
    fn with_name(self, name: impl ToString) -> Result<T, VendorChangeError>;
}
impl<T, E: WithDepName> WithDepNameExt<T> for Result<T, E> {
    fn with_name(self, name: impl ToString) -> Result<T, VendorChangeError> {
        self.map_err(|e| e.with_name_conv(name))
    }
}

impl WithDepName for SerdeError {
    fn with_name(self, name: String) -> VendorChangeError {
        VendorChangeError::SerdeError(name, self)
    }
}

impl Display for SerdeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SerdeError::Toml(e) => write!(f, "{}", e),
            SerdeError::Json(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Error, Debug)]
enum VendorChangeError {
    ConflictingDependency {
        dependency: String,
        versions: VersionsWithSources,
    },
    NoElmJsonFound(String),
    IoError(String, io::Error),
    SerdeError(String, SerdeError),
}

impl WithDepName for serde_json::Error {
    fn with_name(self, name: String) -> VendorChangeError {
        VendorChangeError::SerdeError(name, SerdeError::Json(self))
    }
}

impl WithDepName for io::Error {
    fn with_name(self, name: String) -> VendorChangeError {
        VendorChangeError::IoError(name, self)
    }
}

impl Display for VendorChangeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use VendorChangeError::*;
        match self {
            ConflictingDependency {
                dependency,
                versions,
            } => {
                writeln!(
                    f,
                    "There were dependency specification conflicts for the dependency {}:",
                    dependency
                )?;
                for (source, version) in versions.iter() {
                    writeln!(f, "\t{} wanted {}", source, version)?;
                }
                writeln!(f, "These need to be identical because I'm not smart enough to resolve a version that works for all of them. You may be able to override this soon.")?;
            }
            NoElmJsonFound(dep) => {
                writeln!(f, "There was no elm.json found for the package {}", dep)?
            }
            IoError(dep, err) => writeln!(
                f,
                "There was an IO error while reading the elm json for {}: {}",
                dep, err
            )?,
            SerdeError(dep, e) => {
                writeln!(f, "There was a (de)serialization error for {}: {}", dep, e)?
            }
        }
        Ok(())
    }
}
//endregion

//region version operations
/// source to value
type VersionsWithSources = HashMap<String, String>;

/// dependency name to Source, Value
type DependenciesWithSources = HashMap<String, VersionsWithSources>;

/// Merge all dependency specifications into one if they're identical.
fn collapse_sources(sources: VersionsWithSources) -> Result<String, VersionsWithSources> {
    let values = sources.values().collect::<HashSet<&String>>();
    if values.len() > 1 {
        Err(sources)
    } else {
        Ok(values.into_iter().cloned().next().unwrap())
    }
}
//endregion

//region operations
/// prepend the package path to all source_dirs
fn contextualize_package(relative_root: impl AsRef<Path>, package: &mut ElmPackage) {
    // TODO: maybe this should just work on the dict directly. It might as well be a separate step in extraction, no?
    for source_dir in package.source_dirs.iter_mut() {
        *source_dir = relative_root.as_ref().join(&source_dir);
    }
}

/// Try to load package data from the given package's elm.json
fn load_package_from(package: &str) -> Result<ElmPackage, VendorChangeError> {
    let mut target = env::current_dir()
        .expect("The program couldn't find its own current directory. That's weird.");
    target.push(package);
    let file = fs::File::open(target).map_err(|io_err| match io_err.kind() {
        io::ErrorKind::NotFound => VendorChangeError::NoElmJsonFound(package.to_owned()),
        _ => io_err.with_name_conv(package),
    })?;
    let package_json: ElmPackage = serde_json::from_reader(file).with_name(package.to_owned())?;
    Ok(package_json)
}

fn vendor_change(
    package: ElmPackage,
    vendor: ElmVendor,
) -> Result<ElmPackage, Vec<VendorChangeError>> {
    unimplemented!()
}
//endregion

enum Subprogram {
    /// check if elm.json is committed into git
    /// ask if it's okay
    /// move stuff into elm.json
    Vendor,
    /// try to set elm.json back to its original status as much as possible
    Unvendor,
    /// tests package to see if we should use elm or lamdera,
    /// tries to call one of those to use it,
    /// and then copies that new value into elm-vendor.json
    Install,
    /// extracts direct info to elm-vendor.json
    /// (TODO: we need fields other than the dependencies!)
    Init,
    /// make sure the non-dependency contents of elm.json and elm-vendor.json haven't drifted.
    /// Should be run during CI!
    Check,
}

/// does the given file name match the elm-vendor file name?
fn is_elm_vendor_config_file_name(name: &OsStr) -> bool {
    name == "elm-vendor.json" || name == "elm-vendor.toml"
}

/// Try to find the elm-vendor file in the current directory.
fn find_elm_vendor_json() -> Result<Option<OsString>, anyhow::Error> {
    let mut elm_vendor_files = HashSet::new();
    for dir_ent_res in fs::read_dir(".")? {
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

const INIT_PROMPT: &'static str =
    "I'm going to extract all the user-set fields from elm.json, and add them to elm-vendor.json.";

/// The init command.
fn init() -> Result<(), anyhow::Error> {
    if let Some(_) = find_elm_vendor_json()? {
        anyhow::bail!("An elm-vendor file already exists!");
    }

    let elm_json_file = fs::OpenOptions::new().read(true).open("elm.json")?;
    let elm_json: ElmJson = serde_json::from_reader(elm_json_file)?;

    println!("{}", INIT_PROMPT);
    let ok = Confirm::new().with_prompt("Sound good?").interact()?;
    if !ok {
        return Ok(());
    }

    unimplemented!()
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE_APPLICATION_JSON: &'static str = include_str!("../test-data/sample-elm-app.json");
    const SAMPLE_PACKAGE_JSON: &'static str = include_str!("../test-data/sample-elm-package.json");

    #[test]
    fn test_reading_json() {
        let val: ElmJson = serde_json::from_str(SAMPLE_APPLICATION_JSON).unwrap();
        match val {
            ElmJson::Application(_) => assert!(true),
            _ => assert!(false),
        }
    }
}
