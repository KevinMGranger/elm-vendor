///! Helpers for executing `elm`/`lamdera`.
use anyhow::Result;
use crate::utils::*;
#[allow(unused_imports)]
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, space0},
    combinator::{map, map_res},
    error::ParseError,
    sequence::{delimited, tuple},
    Finish, IResult,
};
use std::ffi::OsStr;
#[allow(unused_imports)]
use std::process::{self, Child, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::io::{BufRead, BufReader, Write};

fn elm_binary(is_lamdera: bool) -> Command {
    Command::new(if is_lamdera { "lamdera" } else { "elm" })
}

fn elm_install(is_lamdera: bool, dependency: impl AsRef<OsStr>) -> Result<()> {
    let mut cmd = elm_binary(is_lamdera);
    cmd.arg("install")
        .arg(dependency)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let mut line = String::new();

    stdout.read_line(&mut line)?;

    here_is_my_plan(&line).finalize()?;

    loop {
        stdout.read_line(&mut line)?;
        if wanna_update(&line).is_ok() {
            break;
        }
    }

    write!(stdin, "\n")?;
    
    //TODO: check output / return status 

    todo!()
}

// Here is my plan:
//   Add:
//     TSFoster/elm-uuid    4.1.0

// Would you like me to update your elm.json accordingly? [Y/n]:

fn here_is_my_plan(input: &str) -> IResult<&str, ()> {
    map(tag("Here is my plan:\n"), |_| ())(input)
}

fn wanna_update(input: &str) -> IResult<&str, ()> {
    map(
        tag("Would you like me to update your elm.json accordingly? [Y/n]:"),
        |_| (),
    )(input)
}
