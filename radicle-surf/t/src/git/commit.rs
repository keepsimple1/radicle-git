use std::str::FromStr;

use proptest::prelude::*;
use radicle_git_ext::Oid;
use radicle_surf::git::{Author, Commit};
use test_helpers::roundtrip;

#[cfg(feature = "serialize")]
proptest! {
    #[test]
    fn prop_test_commits(commit in commits_strategy()) {
        roundtrip::json(commit)
    }
}

fn commits_strategy() -> impl Strategy<Value = Commit> {
    ("[a-fA-F0-9]{40}", any::<String>(), any::<i64>()).prop_map(|(id, text, time)| Commit {
        id: Oid::from_str(&id).unwrap(),
        author: Author {
            name: text.clone(),
            email: text.clone(),
            time: git2::Time::new(time, 0),
        },
        committer: Author {
            name: text.clone(),
            email: text.clone(),
            time: git2::Time::new(time, 0),
        },
        message: text.clone(),
        summary: text,
        parents: vec![Oid::from_str(&id).unwrap(), Oid::from_str(&id).unwrap()],
    })
}
