///! Models for various config files:
/// elm.json (both package and app structure),
/// and elm-vendor.json

use super::version::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// The elm-vendor.{json,toml} config file
#[derive(Deserialize, Serialize)]
pub(crate) struct ElmVendor {
    #[serde(rename = "main-dependencies")]
    pub(crate) main_deps: HashMap<String, DependencyVersion>,
    #[serde(rename = "source-directories")]
    pub(crate) source_dirs: Vec<PathBuf>,
    pub(crate) vendored: Vec<String>,
    #[serde(rename = "type")]
    pub(crate) kind: ElmJsonKind,
    pub(crate) extras: HashMap<String, serde_json::Value>,
}

/// The elm.json file
#[derive(Deserialize, Serialize)]
pub(crate) struct ElmJson {
    #[serde(rename = "source-directories")]
    pub(crate) source_dirs: Vec<PathBuf>,

    #[serde(flatten)]
    pub(crate) dependencies: ElmJsonDeps,

    #[serde(flatten)]
    pub(crate) other_fields: HashMap<String, serde_json::Value>,
}

/// The dependencies section of the elm.json file.
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "dependencies", rename_all = "lowercase")]
pub(crate) enum ElmJsonDeps {
    Application { direct: HashMap<String, SemVer> },
    Package(HashMap<String, DependencyVersion>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ElmJsonKind {
    Application,
    Package,
}

/// The elm.json file, normalized to the important bits we care about.
pub(crate) struct NormalizedElmJson {
    pub(crate) kind: ElmJsonKind,
    pub(crate) source_dirs: Vec<PathBuf>,
    pub(crate) dependencies: HashMap<String, DependencyVersion>,
    pub(crate) other_fields: HashMap<String, serde_json::Value>,
}

impl From<ElmJson> for NormalizedElmJson {
    fn from(json: ElmJson) -> NormalizedElmJson {
        let (kind, deps): (ElmJsonKind, HashMap<String, DependencyVersion>) =
            match json.dependencies {
                ElmJsonDeps::Application { direct } => (
                    ElmJsonKind::Application,
                    direct.into_iter().map(|(k, v)| (k, v.into())).collect(),
                ),
                ElmJsonDeps::Package(deps) => (ElmJsonKind::Package, deps),
            };
        NormalizedElmJson {
            kind: kind,
            source_dirs: json.source_dirs,
            dependencies: deps,
            other_fields: json.other_fields,
        }
    }
}

// impl From<NormalizedElmJson> for ElmJson {
//     fn from(json: NormalizedElmJson) -> ElmJson {
//         ElmJson {
//             source_dirs: json.source_dirs,
//             other_fields: json.other_fields,
//             dependencies: match json.kind {
//                 ElmJsonKind::Application => ElmJsonDeps::Application {
//                     direct: json.dependencies,
//                 },
//                 ElmJsonKind::Package => ElmJsonDeps::Package(json.dependencies),
//             },
//         }
//     }
// }

pub(crate) fn is_lamdera_project<V>(deps: &HashMap<String, V>) -> bool {
    deps.contains_key("lamdera/core")
}
