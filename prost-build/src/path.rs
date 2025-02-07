//! Utilities for working with Protobuf paths.

use std::iter;

use crate::fully_qualified_name::FullyQualifiedName;

/// Maps a fully-qualified Protobuf path to a value using path matchers.
#[derive(Clone, Debug, Default)]
pub(crate) struct PathMap<T> {
    // insertion order might actually matter (to avoid warning about legacy-derive-helpers)
    // see: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#legacy-derive-helpers
    pub(crate) matchers: Vec<(String, T)>,
}

impl<T> PathMap<T> {
    /// Inserts a new matcher and associated value to the path map.
    pub(crate) fn insert(&mut self, matcher: String, value: T) {
        self.matchers.push((matcher, value));
    }

    /// Returns a iterator over all the value matching the given fd_path and associated suffix/prefix path
    pub(crate) fn get(&self, fq_path: &FullyQualifiedName) -> Iter<'_, T> {
        Iter::new(self, fq_path.clone())
    }

    /// Returns a iterator over all the value matching the path `fq_path.field` and associated suffix/prefix path
    pub(crate) fn get_field(&self, fq_path: &FullyQualifiedName, field: &str) -> Iter<'_, T> {
        Iter::new(self, fq_path.join(field))
    }

    /// Returns the first value found matching the given path
    /// If nothing matches the path, suffix paths will be tried, then prefix paths, then the global path
    pub(crate) fn get_first<'a>(&'a self, fq_path: &'_ FullyQualifiedName) -> Option<&'a T> {
        self.find_best_matching(fq_path)
    }

    /// Returns the first value found matching the path `fq_path.field`
    /// If nothing matches the path, suffix paths will be tried, then prefix paths, then the global path
    pub(crate) fn get_first_field<'a>(
        &'a self,
        fq_path: &'_ FullyQualifiedName,
        field: &'_ str,
    ) -> Option<&'a T> {
        self.find_best_matching(&fq_path.join(field))
    }

    /// Removes all matchers from the path map.
    pub(crate) fn clear(&mut self) {
        self.matchers.clear();
    }

    /// Returns the first value found best matching the path
    /// See [sub_path_iter()] for paths test order
    fn find_best_matching(&self, full_path: &FullyQualifiedName) -> Option<&T> {
        sub_path_iter(full_path).find_map(|path| {
            self.matchers
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, v)| v)
        })
    }
}

/// Iterator inside a PathMap that only returns values that matches a given path
pub(crate) struct Iter<'a, T> {
    iter: std::slice::Iter<'a, (String, T)>,
    path: FullyQualifiedName,
}

impl<'a, T> Iter<'a, T> {
    fn new(map: &'a PathMap<T>, path: FullyQualifiedName) -> Self {
        Self {
            iter: map.matchers.iter(),
            path,
        }
    }

    fn is_match(&self, path: &str) -> bool {
        sub_path_iter(&self.path).any(|p| p == path)
    }
}

impl<'a, T> std::iter::Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((p, v)) => {
                    if self.is_match(p) {
                        return Some(v);
                    }
                }
                None => return None,
            }
        }
    }
}

impl<T> std::iter::FusedIterator for Iter<'_, T> {}

/// Given a fully-qualified path, returns a sequence of paths:
/// - the path itself
/// - the sequence of suffix paths
/// - the sequence of prefix paths
/// - the global path
///
/// Example: sub_path_iter(".a.b.c") -> [".a.b.c", "a.b.c", "b.c", "c", ".a.b", ".a", "."]
fn sub_path_iter(full_path: &FullyQualifiedName) -> impl Iterator<Item = &str> {
    let full_path = full_path.as_ref();
    // First, try matching the path.
    iter::once(full_path)
        // Then, try matching path suffixes.
        .chain(suffixes(full_path))
        // Then, try matching path prefixes.
        .chain(prefixes(full_path))
        // Then, match the global path.
        .chain(iter::once("."))
}

/// Given a fully-qualified path, returns a sequence of fully-qualified paths which match a prefix
/// of the input path, in decreasing path-length order.
///
/// Example: prefixes(".a.b.c.d") -> [".a.b.c", ".a.b", ".a"]
fn prefixes(fq_path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(fq_path), |path| {
        #[allow(unknown_lints, clippy::manual_split_once)]
        path.rsplitn(2, '.').nth(1).filter(|path| !path.is_empty())
    })
    .skip(1)
}

/// Given a fully-qualified path, returns a sequence of paths which match the suffix of the input
/// path, in decreasing path-length order.
///
/// Example: suffixes(".a.b.c.d") -> ["a.b.c.d", "b.c.d", "c.d", "d"]
fn suffixes(fq_path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(fq_path), |path| {
        #[allow(unknown_lints, clippy::manual_split_once)]
        path.splitn(2, '.').nth(1).filter(|path| !path.is_empty())
    })
    .skip(1)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_prefixes() {
        assert_eq!(
            prefixes(".a.b.c.d").collect::<Vec<_>>(),
            vec![".a.b.c", ".a.b", ".a"],
        );
        assert_eq!(prefixes(".a").count(), 0);
        assert_eq!(prefixes(".").count(), 0);
    }

    #[test]
    fn test_suffixes() {
        assert_eq!(
            suffixes(".a.b.c.d").collect::<Vec<_>>(),
            vec!["a.b.c.d", "b.c.d", "c.d", "d"],
        );
        assert_eq!(suffixes(".a").collect::<Vec<_>>(), vec!["a"]);
        assert_eq!(suffixes(".").collect::<Vec<_>>(), Vec::<&str>::new());
    }

    #[test]
    fn test_get_matches_sub_path() {
        let mut path_map = PathMap::default();

        let abcd_fqn = FullyQualifiedName::from(".a.b.c.d");
        let abc_fqn = FullyQualifiedName::from(".a.b.c");
        let bcd_fqn = FullyQualifiedName::from("b.c.d");

        // full path
        path_map.insert(".a.b.c.d".to_owned(), 1);
        assert_eq!(Some(&1), path_map.get(&abcd_fqn).next());
        assert_eq!(Some(&1), path_map.get_field(&abc_fqn, "d").next());

        // suffix
        path_map.clear();
        path_map.insert("c.d".to_owned(), 1);
        assert_eq!(Some(&1), path_map.get(&abcd_fqn).next());
        assert_eq!(Some(&1), path_map.get(&bcd_fqn).next());
        assert_eq!(Some(&1), path_map.get_field(&abc_fqn, "d").next());

        // prefix
        path_map.clear();
        path_map.insert(".a.b".to_owned(), 1);
        assert_eq!(Some(&1), path_map.get(&abcd_fqn).next());
        assert_eq!(Some(&1), path_map.get_field(&abc_fqn, "d").next());

        // global
        path_map.clear();
        path_map.insert(".".to_owned(), 1);
        assert_eq!(Some(&1), path_map.get(&abcd_fqn).next());
        assert_eq!(Some(&1), path_map.get(&bcd_fqn).next());
        assert_eq!(Some(&1), path_map.get_field(&abc_fqn, "d").next());
    }

    #[test]
    fn test_get_best() {
        let mut path_map = PathMap::default();

        let abcd_fqn = FullyQualifiedName::from(".a.b.c.d");
        let abc_fqn = FullyQualifiedName::from(".a.b.c");
        let bcd_fqn = FullyQualifiedName::from("b.c.d");

        // worst is global
        path_map.insert(".".to_owned(), 1);
        assert_eq!(Some(&1), path_map.get_first(&abcd_fqn));
        assert_eq!(Some(&1), path_map.get_first(&bcd_fqn));
        assert_eq!(Some(&1), path_map.get_first_field(&abc_fqn, "d"));

        // then prefix
        path_map.insert(".a.b".to_owned(), 2);
        assert_eq!(Some(&2), path_map.get_first(&abcd_fqn));
        assert_eq!(Some(&2), path_map.get_first_field(&abc_fqn, "d"));

        // then suffix
        path_map.insert("c.d".to_owned(), 3);
        assert_eq!(Some(&3), path_map.get_first(&abcd_fqn));
        assert_eq!(Some(&3), path_map.get_first(&bcd_fqn));
        assert_eq!(Some(&3), path_map.get_first_field(&abc_fqn, "d"));

        // best is full path
        path_map.insert(".a.b.c.d".to_owned(), 4);
        assert_eq!(Some(&4), path_map.get_first(&abcd_fqn));
        assert_eq!(Some(&4), path_map.get_first_field(&abc_fqn, "d"));
    }

    #[test]
    fn test_get_keep_order() {
        let abcd_fqn = FullyQualifiedName::from(".a.b.c.d");

        let mut path_map = PathMap::default();
        path_map.insert(".".to_owned(), 1);
        path_map.insert(".a.b".to_owned(), 2);
        path_map.insert(".a.b.c.d".to_owned(), 3);

        let mut iter = path_map.get(&abcd_fqn);
        assert_eq!(Some(&1), iter.next());
        assert_eq!(Some(&2), iter.next());
        assert_eq!(Some(&3), iter.next());
        assert_eq!(None, iter.next());

        path_map.clear();

        path_map.insert(".a.b.c.d".to_owned(), 1);
        path_map.insert(".a.b".to_owned(), 2);
        path_map.insert(".".to_owned(), 3);

        let mut iter = path_map.get(&abcd_fqn);
        assert_eq!(Some(&1), iter.next());
        assert_eq!(Some(&2), iter.next());
        assert_eq!(Some(&3), iter.next());
        assert_eq!(None, iter.next());
    }
}
