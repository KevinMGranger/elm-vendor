// mod shared;
// mod utils;

use elm_vendor::CmdContext;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
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

#[derive(StructOpt)]
struct Program {
    #[structopt(short, long)]
    yes: bool,

    #[structopt(subcommand)]
    cmd: Subprogram,
}

fn main() {
    let args = Program::from_args();

    let ctx = CmdContext {
        yes: args.yes,
        root: PathBuf::from("."),
    };

    match args.cmd {
        Subprogram::Vendor => ctx.vendor().unwrap(),
        _ => {
            println!("not yet")
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     const SAMPLE_APPLICATION_JSON: &'static str = include_str!("../test-data/sample-elm-app.json");
//     // const SAMPLE_PACKAGE_JSON: &'static str = include_str!("../test-data/sample-elm-package.json");

//     // #[test]
//     // fn test_reading_json() {
//     //     let val: ElmJson = serde_json::from_str(SAMPLE_APPLICATION_JSON).unwrap();
//     //     match val {
//     //         ElmJson::Application(_) => assert!(true),
//     //         _ => assert!(false),
//     //     }
//     // }
//     // #[test]
//     // fn test_reading_json() {
//     //     let val: ElmJson = serde_json::from_str(SAMPLE_APPLICATION_JSON).unwrap();
//     //     match val.dependencies {
//     //         ElmJsonDeps::Application { .. } => assert!(true),
//     //         ElmJsonDeps::Package(_) => assert!(false),
//     //     }
//     // }
// }
