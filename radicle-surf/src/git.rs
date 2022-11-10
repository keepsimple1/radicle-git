// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//! use std::str::FromStr;
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let repo = Repository::new("./data/git-platinum")?;
//!
//! // Pin the browser to a parituclar commit.
//! let pin_commit = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
//! let mut browser = Browser::new(&repo, Branch::local("master"))?;
//! browser.commit(pin_commit)?;
//!
//! let directory = browser.get_directory()?;
//! let mut directory_contents = directory.list_directory();
//! directory_contents.sort();
//!
//! assert_eq!(directory_contents, vec![
//!     SystemType::file(unsound::label::new(".i-am-well-hidden")),
//!     SystemType::file(unsound::label::new(".i-too-am-hidden")),
//!     SystemType::file(unsound::label::new("README.md")),
//!     SystemType::directory(unsound::label::new("bin")),
//!     SystemType::directory(unsound::label::new("src")),
//!     SystemType::directory(unsound::label::new("text")),
//!     SystemType::directory(unsound::label::new("this")),
//! ]);
//!
//! // find src directory in the Git directory and the in-memory directory
//! let src_directory = directory
//!     .find_directory(Path::new(unsound::label::new("src")))
//!     .expect("failed to find src");
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! #
//! # Ok(())
//! # }
//! ```

use std::str::FromStr;

// Re-export git2 as sub-module
pub use git2::{self, Error as Git2Error, Time};
use git_ref_format::{name::Components, Component, Qualified, RefString};
pub use radicle_git_ext::Oid;

mod repo;
pub use repo::{Repository, RepositoryRef};

mod glob;
pub use glob::Glob;

mod history;
pub use history::History;

pub mod error;
pub use error::Error;

/// Provides the data for talking about branches.
pub mod branch;
pub use branch::{Branch, Local, Remote};

/// Provides the data for talking about tags.
pub mod tag;
pub use tag::Tag;

/// Provides the data for talking about commits.
pub mod commit;
pub use commit::{Author, Commit};

/// Provides the data for talking about namespaces.
pub mod namespace;
pub use namespace::Namespace;

/// Provides the data for talking about repository statistics.
pub mod stats;
pub use stats::Stats;

pub use crate::diff::Diff;

/// The signature of a commit
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signature(Vec<u8>);

impl From<git2::Buf> for Signature {
    fn from(other: git2::Buf) -> Self {
        Signature((*other).into())
    }
}

/// Supports various ways to specify a revision used in Git.
pub trait Revision {
    /// Returns the object id of this revision in `repo`.
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error>;
}

impl Revision for RefString {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        repo.refname_to_oid(self.as_str())
    }
}

impl Revision for &RefString {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        repo.refname_to_oid(self.as_str())
    }
}

impl Revision for Qualified<'_> {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        repo.refname_to_oid(self.as_str())
    }
}

impl Revision for &Qualified<'_> {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        repo.refname_to_oid(self.as_str())
    }
}

impl Revision for Oid {
    fn object_id(&self, _repo: &RepositoryRef) -> Result<Oid, Error> {
        Ok(*self)
    }
}

impl Revision for &str {
    fn object_id(&self, _repo: &RepositoryRef) -> Result<Oid, Error> {
        Oid::from_str(self).map_err(Error::Git)
    }
}

impl Revision for &Branch {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        let refname = repo.namespaced_refname(&self.refname())?;
        Ok(repo.repo_ref.refname_to_id(&refname).map(Oid::from)?)
    }
}

impl Revision for &Tag {
    fn object_id(&self, repo: &RepositoryRef) -> Result<Oid, Error> {
        let refname = repo.namespaced_refname(&self.refname())?;
        Ok(repo.repo_ref.refname_to_id(&refname).map(Oid::from)?)
    }
}

pub(crate) fn refstr_join<'a>(c: Component<'a>, cs: Components<'a>) -> RefString {
    std::iter::once(c).chain(cs).collect::<RefString>()
}
