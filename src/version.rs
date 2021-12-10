///! Data structures for elm's flavor of semver.
///
/// Includes parsing, serializing,
/// and ways to represent semver within a `ranges::Domain`,
/// which should help with dependency reconciliation.
use crate::utils::*;
use fehler::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, space0},
    combinator::{map, map_res},
    error::ParseError,
    sequence::{delimited, tuple},
    IResult,
};
use ranges::{Domain, GenericRange};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::ops::{Bound, RangeBounds};
use std::str::FromStr;
use std::u64;

//region SemVer
#[derive(
    PartialOrd, Ord, PartialEq, Eq, Clone, Copy, DeserializeFromStr, SerializeDisplay, Debug,
)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Domain for SemVer {
    const DISCRETE: bool = true;

    #[throws(as Option)]
    fn predecessor(&self) -> Self {
        if self.patch != 0 {
            return SemVer {
                patch: self.patch - 1,
                ..*self
            };
        } else if self.minor != 0 {
            return SemVer {
                patch: u64::MAX,
                minor: self.minor - 1,
                major: self.major,
            };
        } else if self.major != 0 {
            return SemVer {
                patch: u64::MAX,
                minor: u64::MAX,
                major: self.major - 1,
            };
        } else {
            throw!();
        }
    }
    #[throws(as Option)]
    fn successor(&self) -> Self {
        if self.patch != u64::MAX {
            return SemVer {
                patch: self.patch + 1,
                ..*self
            };
        } else if self.minor != u64::MAX {
            return SemVer {
                patch: 0,
                minor: self.minor + 1,
                major: self.major,
            };
        } else if self.major != u64::MAX {
            return SemVer {
                patch: 0,
                minor: 0,
                major: self.major + 1,
            };
        } else {
            throw!();
        }
    }
    // fn predecessor(&self) -> Option<Self> {
    //     Some(match self.patch.checked_sub(1) {
    //         Some(patch) => SemVer { patch, ..*self },
    //         None => match self.minor.checked_sub(1) {
    //             Some(minor) => SemVer {
    //                 minor,
    //                 patch: u64::MAX,
    //                 ..*self
    //             },
    //             None => match self.major.checked_sub(1) {
    //                 Some(major) => SemVer {
    //                     major,
    //                     minor: u64::MAX,
    //                     patch: u64::MAX,
    //                 },
    //                 None => return None,
    //             },
    //         },
    //     })
    // }
    // fn successor(&self) -> Option<Self> {
    //     Some(match self.patch.checked_add(1) {
    //         Some(patch) => SemVer { patch, ..*self },
    //         None => match self.minor.checked_add(1) {
    //             Some(minor) => SemVer {
    //                 minor,
    //                 patch: u64::MIN,
    //                 ..*self
    //             },
    //             None => match self.major.checked_add(1) {
    //                 Some(major) => SemVer {
    //                     major,
    //                     minor: u64::MIN,
    //                     patch: u64::MIN,
    //                 },
    //                 None => return None,
    //             },
    //         },
    //     })
    // }
    fn minimum() -> Bound<Self> {
        Bound::Included(SemVer {
            major: 0,
            minor: 0,
            patch: 0,
        })
    }
    fn maximum() -> Bound<Self> {
        Bound::Included(SemVer {
            major: u64::MAX,
            minor: u64::MAX,
            patch: u64::MAX,
        })
    }
}

impl Display for SemVer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemVer {
    type Err = nom::error::Error<String>;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        semver(str).finalize()
    }
}
//endregion

//region Relation
#[derive(Debug, Clone, Copy)]
pub enum Relation {
    LTE,
    LT,
}

impl Display for Relation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Relation::LTE => write!(f, "<="),
            Relation::LT => write!(f, "<"),
        }
    }
}
//endregion

//region VersionRange
#[derive(Debug, Clone)]
pub struct VersionRange {
    pub lower: SemVer,
    pub lower_relation: Relation,
    pub higher_relation: Relation,
    pub higher: SemVer,
}

impl Into<GenericRange<SemVer>> for &VersionRange {
    fn into(self) -> GenericRange<SemVer> {
        GenericRange::new_with_bounds(self.start_bound().cloned(), self.end_bound().cloned())
    }
}

impl From<GenericRange<SemVer>> for VersionRange {
    fn from(range: GenericRange<SemVer>) -> VersionRange {
        let (lower, lower_relation) = match range.start_bound().cloned() {
            Bound::Included(x) => (x, Relation::LTE),
            Bound::Excluded(x) => (x, Relation::LT),
            Bound::Unbounded => panic!("Unbounded range should never occur for version range"),
        };
        let (higher, higher_relation) = match range.end_bound().cloned() {
            Bound::Included(x) => (x, Relation::LTE),
            Bound::Excluded(x) => (x, Relation::LT),
            Bound::Unbounded => panic!("Unbounded range should never occur for version range"),
        };

        VersionRange {
            lower,
            lower_relation,
            higher_relation,
            higher,
        }
    }
}

impl Display for VersionRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} v {} {}",
            self.lower, self.lower_relation, self.higher_relation, self.higher
        )
    }
}

impl RangeBounds<SemVer> for VersionRange {
    fn start_bound(&self) -> Bound<&SemVer> {
        match self.lower_relation {
            Relation::LTE => Bound::Included(&self.lower),
            Relation::LT => Bound::Excluded(&self.lower),
        }
    }
    fn end_bound(&self) -> Bound<&SemVer> {
        match self.higher_relation {
            Relation::LTE => Bound::Included(&self.higher),
            Relation::LT => Bound::Excluded(&self.higher),
        }
    }
}

impl FromStr for VersionRange {
    type Err = nom::error::Error<String>;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        version_range(str).finalize()
    }
}
//endregion

//region DepVersion
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub enum DependencyVersion {
    SpecificVersion(SemVer),
    VersionRange(VersionRange),
}

impl From<SemVer> for DependencyVersion {
    fn from(ver: SemVer) -> Self {
        DependencyVersion::SpecificVersion(ver)
    }
}

impl Display for DependencyVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DependencyVersion::SpecificVersion(x) => x.fmt(f),
            DependencyVersion::VersionRange(x) => x.fmt(f),
        }
    }
}

impl Into<GenericRange<SemVer>> for &DependencyVersion {
    fn into(self) -> GenericRange<SemVer> {
        match self {
            DependencyVersion::SpecificVersion(x) => GenericRange::singleton(x.clone()),
            DependencyVersion::VersionRange(x) => x.into(),
        }
    }
}

impl TryFrom<GenericRange<SemVer>> for DependencyVersion {
    type Error = ();

    fn try_from(range: GenericRange<SemVer>) -> Result<DependencyVersion, ()> {
        Ok(if range.is_singleton() {
            DependencyVersion::SpecificVersion(match range.start_bound() {
                Bound::Included(x) => *x,
                _ => unreachable!(),
            })
        } else if range.is_empty() {
            return Err(());
        } else {
            DependencyVersion::VersionRange(range.into())
        })
    }
}

impl FromStr for DependencyVersion {
    type Err = nom::error::Error<String>;

    fn from_str(str: &str) -> Result<DependencyVersion, Self::Err> {
        version(str).finalize()
    }
}
//endregion

//region parsers
/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(space0, inner, space0)
}

fn u64(input: &str) -> IResult<&str, u64> {
    map_res(digit1, u64::from_str)(input)
}

fn semver(input: &str) -> IResult<&str, SemVer> {
    map(
        tuple((u64, tag("."), u64, tag("."), u64)),
        |(major, _, minor, _, patch)| SemVer {
            major,
            minor,
            patch,
        },
    )(input)
}

fn relation(input: &str) -> IResult<&str, Relation> {
    let lt = map(tag("<"), |_| Relation::LT);
    let lte = map(tag("<="), |_| Relation::LTE);

    alt((lt, lte))(input)
}

fn version_variable(input: &str) -> IResult<&str, ()> {
    map(tag("v"), |_| ())(input)
}

fn version_range(input: &str) -> IResult<&str, VersionRange> {
    map(
        tuple((
            ws(semver),
            ws(relation),
            ws(version_variable),
            ws(relation),
            ws(semver),
        )),
        |(lower, lower_relation, _, higher_relation, higher)| VersionRange {
            lower,
            lower_relation,
            higher_relation,
            higher,
        },
    )(input)
}

fn version(input: &str) -> IResult<&str, DependencyVersion> {
    let semver = map(semver, DependencyVersion::SpecificVersion);
    let range = map(version_range, DependencyVersion::VersionRange);

    alt((range, semver))(input)
}
//endregion
