///! Vendoring operations.

use crate::shared::*;
use crate::utils::*;
use crate::version::*;
use anyhow::{ensure, Context, Result};
use ranges::{GenericRange, Ranges};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::string::ToString;
use tap::Conv;
use thiserror::Error;

// region error handling

#[derive(Debug)]
pub(crate) enum SerdeError {
    #[allow(dead_code)]
    Toml(toml::de::Error),
    Json(serde_json::Error),
}

/// Convenience trait to attach a dependency name to an error.
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

/// An error that occurred during the vendoring process.
#[derive(Error, Debug)]
pub(crate) enum VendorChangeError {
    /// Various vendored packages requested incompatible
    /// versions of a dependency.
    ConflictingDependency {
        dependency: String,
        versions: VersionsWithSources,
    },
    /// The vendored package didn't have an elm.json!
    NoElmJsonFound(String),
    /// Computer broke
    IoError(String, io::Error),
    /// The structure of a file wasn't quite what we expected.
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

impl WithDepName for VersionsWithSources {
    fn with_name(self, dependency: String) -> VendorChangeError {
        VendorChangeError::ConflictingDependency {
            dependency,
            versions: self,
        }
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

/// source to value
type VersionsWithSources = HashMap<String, DependencyVersion>;

/// dependency name to Source, Value
type DependenciesWithSources = HashMap<String, VersionsWithSources>;

struct Dependency {
    /// The package name that this dependency came from
    source: String,
    /// The dependency name
    dependency: String,
    /// The version of the dependency
    version: DependencyVersion,
}

/// Merge all dependency specifications into one if they're identical.
fn collapse_sources(
    sources: VersionsWithSources,
) -> Result<DependencyVersion, VersionsWithSources> {
    let ranges: Ranges<SemVer> = sources
        .values()
        .map(|version| version.conv::<GenericRange<SemVer>>())
        .collect();
    let range_slice = ranges.as_slice();
    if range_slice.len() != 1 {
        Err(sources)
    } else {
        Ok(range_slice[0].try_into().unwrap())
    }
}

/// Take the flattened list of dependencies with sources
/// and create a dict with it.
fn coalesce_dependencies(
    dependencies: impl IntoIterator<Item = Dependency>,
    deps_with_sources: &mut DependenciesWithSources,
) {
    // maybe this'll someday be beautiful with iterator methods. Someday.
    for Dependency {
        source,
        dependency,
        version,
    } in dependencies
    {
        deps_with_sources
            .entry(dependency)
            .or_default()
            .insert(source, version);
    }
}

impl NormalizedElmJson {
    /// extract all paths to source dirs, contextualized by the package's name
    fn contextualize_source_dirs<'a>(
        &'a self,
        name: &'a Path,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        self.source_dirs
            .iter()
            .map(move |source_dir| name.join(source_dir))
    }

    fn dependencies_with_source_name<'a>(
        &'a self,
        package: &'a str,
    ) -> impl Iterator<Item = Dependency> + 'a {
        self.dependencies
            .iter()
            .map(move |(dependency, version)| Dependency {
                source: package.to_owned(),
                dependency: dependency.clone(),
                version: version.clone(),
            })
    }
}

// fn vendor_change(
//     package: ElmPackage,
//     vendor: ElmVendor,
// ) -> Result<ElmPackage, Vec<VendorChangeError>> {
//     unimplemented!()
// }

impl super::CmdContext {
    pub(crate) fn load_package_for(
        &self,
        package: &str,
    ) -> Result<NormalizedElmJson, VendorChangeError> {
        let file =
            fs::File::open(self.root.join(package).join("elm.json")).map_err(
                |io_err| match io_err.kind() {
                    io::ErrorKind::NotFound => {
                        VendorChangeError::NoElmJsonFound(package.to_owned())
                    }
                    _ => io_err.with_name_conv(package),
                },
            )?;
        let package_json: ElmJson = serde_json::from_reader(file).with_name(package.to_owned())?;
        Ok(package_json.into())
    }

    pub fn vendor(&self) -> Result<()> {
        let is_committed = self.check_if_elm_json_is_commited()?;

        ensure!(is_committed, "elm.json is not committed!");
        let elm_vendor_json_name = self
            .find_elm_vendor_json()?
            // TODO do we just run it for them?
            .context("you must run elm-vendor init first")?;

        let elm_vendor_json_file = fs::OpenOptions::new()
            .read(true)
            .open(self.root.join(elm_vendor_json_name))?;
        let elm_vendor_json: ElmVendor = serde_json::from_reader(elm_vendor_json_file)?;
        let is_lamdera_project = is_lamdera_project(&elm_vendor_json.main_deps);

        let results: Vec<(Vec<PathBuf>, Vec<Dependency>)> = elm_vendor_json
            .vendored
            .iter()
            .try_with_progress(|vendored_pkg| -> Result<_> {
                let package_json = self.load_package_for(vendored_pkg)?;

                let source_dirs = package_json.contextualize_source_dirs(vendored_pkg.as_ref());

                let dependencies = package_json.dependencies_with_source_name(vendored_pkg);

                Ok((source_dirs.collect(), dependencies.collect()))
            })
            .map_err(MultiError::from)?;
        let mut source_dirs = elm_vendor_json.source_dirs;

        let mut dependencies = elm_vendor_json
            .main_deps
            .into_iter()
            .map(|(dependency, version)| -> Result<_> {
                let mut deps = HashMap::new();
                deps.insert("main package (elm-vendor.json)".to_owned(), version);

                Ok((dependency, deps))
            })
            .collect::<Result<DependenciesWithSources, _>>()?;

        for (vendored_pkg_source_dirs, vendored_pkg_deps) in results {
            source_dirs.extend(vendored_pkg_source_dirs);

            coalesce_dependencies(vendored_pkg_deps, &mut dependencies)
        }

        let dependencies: HashMap<String, DependencyVersion> = dependencies
            .into_iter()
            .try_with_progress(|(dependency, sources)| -> Result<_> {
                let version = collapse_sources(sources).with_name(&dependency)?;
                Ok((dependency, version))
            })
            .map_err(MultiError::from)
            .map(HashMap::from_iter)?;

        Ok(())
    }
}
