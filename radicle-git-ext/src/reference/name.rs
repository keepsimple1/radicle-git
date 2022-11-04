// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{
    convert::TryFrom,
    fmt::{self, Display},
    iter::FromIterator,
    ops::Deref,
    path::Path,
    str::{self, FromStr},
};

pub use percent_encoding::PercentEncode;
use thiserror::Error;

use super::check;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid utf8")]
    Utf8,

    #[error("not a valid git ref name or pattern")]
    RefFormat(#[from] check::Error),
}

impl Error {
    pub const fn empty() -> Self {
        Self::RefFormat(check::Error::Empty)
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StripPrefixError {
    #[error("prefix is equal to path")]
    ImproperPrefix,

    #[error("not prefixed by given path")]
    NotPrefix,
}

/// An owned path-like value which is a valid git refname.
///
/// See [`git-check-ref-format`] for what the rules for refnames are --
/// conversion functions behave as if `--allow-onelevel` was given.
/// Additionally, we impose the rule that the name must consist of valid utf8.
///
/// Note that refspec patterns (eg. "refs/heads/*") are not allowed (see
/// [`RefspecPattern`]), and that the maximum length of the name is 1024 bytes.
///
/// [`git-check-ref-format`]: https://git-scm.com/docs/git-check-ref-format
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "String")
)]
pub struct RefLike(String);

impl RefLike {
    /// Append the path in `Other` to `self.
    pub fn join<Other: Into<Self>>(&self, other: Other) -> Self {
        Self(format!("{}/{}", self.0, other.into().0))
    }

    /// Append a [`RefspecPattern`], yielding a [`RefspecPattern`]
    pub fn with_pattern_suffix<Suf: Into<RefspecPattern>>(&self, suf: Suf) -> RefspecPattern {
        RefspecPattern(format!("{}/{}", self.0, suf.into().0))
    }

    /// Returns a [`RefLike`] that, when joined onto `base`, yields `self`.
    ///
    /// # Errors
    ///
    /// If `base` is not a prefix of `self`, or `base` equals the path in `self`
    /// (ie. the result would be the empty path, which is not a valid
    /// [`RefLike`]).
    pub fn strip_prefix<P: AsRef<str>>(&self, base: P) -> Result<Self, StripPrefixError> {
        let base = base.as_ref();
        let base = format!("{}/", base.strip_suffix('/').unwrap_or(base));
        self.0
            .strip_prefix(&base)
            .ok_or(StripPrefixError::NotPrefix)
            .and_then(|path| {
                if path.is_empty() {
                    Err(StripPrefixError::ImproperPrefix)
                } else {
                    Ok(Self(path.into()))
                }
            })
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn percent_encode(&self) -> PercentEncode {
        /// https://url.spec.whatwg.org/#fragment-percent-encode-set
        const FRAGMENT_PERCENT_ENCODE_SET: &percent_encoding::AsciiSet =
            &percent_encoding::CONTROLS
                .add(b' ')
                .add(b'"')
                .add(b'<')
                .add(b'>')
                .add(b'`');

        /// https://url.spec.whatwg.org/#path-percent-encode-set
        const PATH_PERCENT_ENCODE_SET: &percent_encoding::AsciiSet = &FRAGMENT_PERCENT_ENCODE_SET
            .add(b'#')
            .add(b'?')
            .add(b'{')
            .add(b'}');

        percent_encoding::utf8_percent_encode(self.as_str(), PATH_PERCENT_ENCODE_SET)
    }
}

impl Deref for RefLike {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for RefLike {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for RefLike {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        check::ref_format(
            check::Options {
                allow_onelevel: true,
                allow_pattern: false,
            },
            s,
        )?;
        Ok(Self(s.to_owned()))
    }
}

impl TryFrom<&[u8]> for RefLike {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(bytes)
            .or(Err(Error::Utf8))
            .and_then(Self::try_from)
    }
}

impl FromStr for RefLike {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<String> for RefLike {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl TryFrom<&Path> for RefLike {
    type Error = Error;

    #[cfg(target_family = "windows")]
    fn try_from(p: &Path) -> Result<Self, Self::Error> {
        use std::{convert::TryInto as _, path::Component::Normal};

        p.components()
            .filter_map(|comp| match comp {
                Normal(s) => Some(s),
                _ => None,
            })
            .map(|os| os.to_str().ok_or(Error::Utf8))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join("/")
            .try_into()
    }

    #[cfg(target_family = "unix")]
    fn try_from(p: &Path) -> Result<Self, Self::Error> {
        Self::try_from(p.to_str().ok_or(Error::Utf8)?)
    }
}

impl From<&RefLike> for RefLike {
    fn from(me: &RefLike) -> Self {
        me.clone()
    }
}

impl From<git_ref_format::RefString> for RefLike {
    #[inline]
    fn from(r: git_ref_format::RefString) -> Self {
        Self(r.into())
    }
}

impl From<&git_ref_format::RefString> for RefLike {
    #[inline]
    fn from(r: &git_ref_format::RefString) -> Self {
        Self::from(r.as_refstr())
    }
}

impl From<&git_ref_format::RefStr> for RefLike {
    #[inline]
    fn from(r: &git_ref_format::RefStr) -> Self {
        Self(r.to_owned().into())
    }
}

impl From<RefLike> for String {
    fn from(RefLike(path): RefLike) -> Self {
        path
    }
}

impl FromIterator<Self> for RefLike {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Self>,
    {
        Self(iter.into_iter().map(|x| x.0).collect::<Vec<_>>().join("/"))
    }
}

impl Display for RefLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A [`RefLike`] without a "refs/" prefix.
///
/// Conversion functions strip the first **two** path components iff the path
/// starts with `refs/`.
///
/// Note that the [`serde::Deserialize`] impl thusly implies that input in
/// [`Qualified`] form is accepted, and silently converted.
///
/// # Examples
///
/// ```rust
/// use std::convert::TryFrom;
/// use radicle_git_ext::reference::name::*;
///
/// assert_eq!(
///     &*OneLevel::from(RefLike::try_from("refs/heads/next").unwrap()),
///     "next"
/// );
///
/// assert_eq!(
///     &*OneLevel::from(RefLike::try_from("refs/remotes/origin/it").unwrap()),
///     "origin/it"
/// );
///
/// assert_eq!(
///     &*OneLevel::from(RefLike::try_from("mistress").unwrap()),
///     "mistress"
/// );
///
/// assert_eq!(
///     OneLevel::from_qualified(Qualified::from(RefLike::try_from("refs/tags/grace").unwrap())),
///     (
///         OneLevel::from(RefLike::try_from("grace").unwrap()),
///         Some(RefLike::try_from("tags").unwrap())
///     ),
/// );
///
/// assert_eq!(
///     OneLevel::from_qualified(Qualified::from(RefLike::try_from("refs/remotes/origin/hopper").unwrap())),
///     (
///         OneLevel::from(RefLike::try_from("origin/hopper").unwrap()),
///         Some(RefLike::try_from("remotes").unwrap())
///     ),
/// );
///
/// assert_eq!(
///     OneLevel::from_qualified(Qualified::from(RefLike::try_from("refs/HEAD").unwrap())),
///     (OneLevel::from(RefLike::try_from("HEAD").unwrap()), None)
/// );
///
/// assert_eq!(
///     &*OneLevel::from(RefLike::try_from("origin/hopper").unwrap()).into_qualified(
///         RefLike::try_from("remotes").unwrap()
///     ),
///     "refs/remotes/origin/hopper",
/// );
/// ```
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "RefLike")
)]
pub struct OneLevel(String);

impl OneLevel {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn from_qualified(Qualified(path): Qualified) -> (Self, Option<RefLike>) {
        let mut path = path.strip_prefix("refs/").unwrap_or(&path).split('/');
        match path.next() {
            Some(category) => {
                let category = RefLike(category.into());
                // check that the "category" is not the only component of the path
                match path.next() {
                    Some(head) => (
                        Self(
                            std::iter::once(head)
                                .chain(path)
                                .collect::<Vec<_>>()
                                .join("/"),
                        ),
                        Some(category),
                    ),
                    None => (Self::from(category), None),
                }
            },
            None => unreachable!(),
        }
    }

    pub fn into_qualified(self, category: RefLike) -> Qualified {
        Qualified(format!("refs/{}/{}", category, self))
    }
}

impl Deref for OneLevel {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for OneLevel {
    fn as_ref(&self) -> &str {
        self
    }
}

impl From<RefLike> for OneLevel {
    fn from(RefLike(path): RefLike) -> Self {
        if path.starts_with("refs/") {
            Self(path.split('/').skip(2).collect::<Vec<_>>().join("/"))
        } else {
            Self(path)
        }
    }
}

impl From<Qualified> for OneLevel {
    fn from(Qualified(path): Qualified) -> Self {
        Self::from(RefLike(path))
    }
}

impl From<OneLevel> for RefLike {
    fn from(OneLevel(path): OneLevel) -> Self {
        Self(path)
    }
}

impl From<OneLevel> for String {
    fn from(OneLevel(path): OneLevel) -> Self {
        path
    }
}

impl Display for OneLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A [`RefLike`] **with** a "refs/" prefix.
///
/// Conversion functions will assume `refs/heads/` if the input was not
/// qualified.
///
/// Note that the [`serde::Deserialize`] impl thusly implies that input in
/// [`OneLevel`] form is accepted, and silently converted.
///
/// # Examples
///
/// ```rust
/// use std::convert::TryFrom;
/// use radicle_git_ext::reference::name::*;
///
/// assert_eq!(
///     &*Qualified::from(RefLike::try_from("laplace").unwrap()),
///     "refs/heads/laplace"
/// );
///
/// assert_eq!(
///     &*Qualified::from(RefLike::try_from("refs/heads/pu").unwrap()),
///     "refs/heads/pu"
/// );
///
/// assert_eq!(
///     &*Qualified::from(RefLike::try_from("refs/tags/v6.6.6").unwrap()),
///     "refs/tags/v6.6.6"
/// );
/// ```
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "RefLike")
)]
pub struct Qualified(String);

impl Qualified {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Qualified {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Qualified {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<RefLike> for Qualified {
    fn from(RefLike(path): RefLike) -> Self {
        if path.starts_with("refs/") {
            Self(path)
        } else {
            Self(format!("refs/heads/{}", path))
        }
    }
}

impl From<OneLevel> for Qualified {
    fn from(OneLevel(path): OneLevel) -> Self {
        Self::from(RefLike(path))
    }
}

impl From<Qualified> for RefLike {
    fn from(Qualified(path): Qualified) -> Self {
        Self(path)
    }
}

impl From<Qualified> for String {
    fn from(Qualified(path): Qualified) -> Self {
        path
    }
}

impl Display for Qualified {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self)
    }
}

/// An owned, path-like value which is a valid refspec pattern.
///
/// Conversion functions behave as if `--allow-onelevel --refspec-pattern` where
/// given to [`git-check-ref-format`]. That is, most of the rules of [`RefLike`]
/// apply, but the path _may_ contain exactly one `*` character.
///
/// [`git-check-ref-format`]: https://git-scm.com/docs/git-check-ref-format
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "String")
)]
pub struct RefspecPattern(String);

impl RefspecPattern {
    /// Append the `RefLike` to the `RefspecPattern`. This allows the creation
    /// of patterns where the `*` appears in the middle of the path, e.g.
    /// `refs/remotes/*/mfdoom`
    pub fn append(&self, refl: impl Into<RefLike>) -> Self {
        RefspecPattern(format!("{}/{}", self.0, refl.into()))
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl From<&RefspecPattern> for RefspecPattern {
    fn from(pat: &RefspecPattern) -> Self {
        pat.clone()
    }
}

impl Deref for RefspecPattern {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for RefspecPattern {
    fn as_ref(&self) -> &str {
        self
    }
}

impl TryFrom<&str> for RefspecPattern {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        check::ref_format(
            check::Options {
                allow_onelevel: true,
                allow_pattern: true,
            },
            s,
        )?;
        Ok(Self(s.to_owned()))
    }
}

impl TryFrom<&[u8]> for RefspecPattern {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(bytes)
            .or(Err(Error::Utf8))
            .and_then(Self::try_from)
    }
}

impl FromStr for RefspecPattern {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<String> for RefspecPattern {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl From<RefspecPattern> for String {
    fn from(RefspecPattern(path): RefspecPattern) -> Self {
        path
    }
}

impl Display for RefspecPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// `RefLike`-likes can be coerced into `RefspecPattern`s

impl From<RefLike> for RefspecPattern {
    fn from(RefLike(path): RefLike) -> Self {
        Self(path)
    }
}

impl From<&RefLike> for RefspecPattern {
    fn from(RefLike(path): &RefLike) -> Self {
        Self(path.to_owned())
    }
}

impl From<OneLevel> for RefspecPattern {
    fn from(OneLevel(path): OneLevel) -> Self {
        Self(path)
    }
}

impl From<&OneLevel> for RefspecPattern {
    fn from(OneLevel(path): &OneLevel) -> Self {
        Self(path.to_owned())
    }
}

impl From<Qualified> for RefspecPattern {
    fn from(Qualified(path): Qualified) -> Self {
        Self(path)
    }
}

impl From<&Qualified> for RefspecPattern {
    fn from(Qualified(path): &Qualified) -> Self {
        Self(path.to_owned())
    }
}

impl From<git_ref_format::refspec::PatternString> for RefspecPattern {
    #[inline]
    fn from(r: git_ref_format::refspec::PatternString) -> Self {
        Self(r.into())
    }
}

impl From<&git_ref_format::refspec::PatternStr> for RefspecPattern {
    #[inline]
    fn from(r: &git_ref_format::refspec::PatternStr) -> Self {
        Self(r.to_owned().into())
    }
}
