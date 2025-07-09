use std::collections::HashSet;

use pgrx::{prelude::*, Uuid};

fn array_union_generic<T>(
    a: impl IntoIterator<Item = Option<T>>,
    b: impl IntoIterator<Item = Option<T>>,
) -> Vec<T>
where
    T: Eq + std::hash::Hash,
{
    // silently discard NULLs
    let mut a: HashSet<_> = a.into_iter().flatten().collect();
    let b_iter = b.into_iter().flatten();
    a.extend(b_iter);
    a.into_iter().collect()
}

fn array_union_generic_sorted<T>(
    a: impl IntoIterator<Item = Option<T>>,
    b: impl IntoIterator<Item = Option<T>>,
) -> Vec<T>
where
    T: Eq + std::hash::Hash + Ord,
{
    let mut s = array_union_generic(a, b);
    s.sort();
    s
}

#[pg_extern(name = "array_union", parallel_safe, immutable, create_or_replace)]
fn array_union_uuid(a: Option<Vec<Option<Uuid>>>, b: Option<Vec<Option<Uuid>>>) -> Vec<Uuid> {
    array_union_generic(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
    )
}

#[pg_extern(name = "array_union", parallel_safe, immutable, create_or_replace)]
fn array_union_text(a: Option<Vec<Option<String>>>, b: Option<Vec<Option<String>>>) -> Vec<String> {
    array_union_generic(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
    )
}

#[pg_extern(name = "array_union", parallel_safe, immutable, create_or_replace)]
fn array_union_i64(a: Option<Vec<Option<i64>>>, b: Option<Vec<Option<i64>>>) -> Vec<i64> {
    array_union_generic(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
    )
}

#[pg_extern(name = "array_union", parallel_safe, immutable, create_or_replace)]
fn array_union_i32(a: Option<Vec<Option<i32>>>, b: Option<Vec<Option<i32>>>) -> Vec<i32> {
    array_union_generic(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
    )
}

#[pg_extern(name = "array_union", parallel_safe, immutable, create_or_replace)]
fn array_union_sorted(a: Option<Vec<Option<Uuid>>>, b: Option<Vec<Option<Uuid>>>) -> Vec<Uuid> {
    array_union_generic_sorted(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test_array_union_generic() {
        let test_cases = [
            (
                vec![1, 2, 3, 4, 5],
                vec![1, 2, 5, 6],
                vec![1, 2, 3, 4, 5, 6],
            ),
            (vec![3, 4, 5], vec![3], vec![3, 4, 5]),
            (vec![], vec![1], vec![1]),
            (vec![1], vec![], vec![1]),
            (vec![], vec![], vec![]),
        ];
        for (a, b, golden) in test_cases {
            let a = a.into_iter().map(Option::Some);
            let b = b.into_iter().map(Option::Some);
            let mut result = super::array_union_generic(a, b);
            result.sort();

            assert_eq!(result, golden);
        }
    }

    #[test]
    pub fn test_array_union_generic_sorted() {
        let test_cases = [
            (
                vec![1, 4, 3, 2, 5],
                vec![1, 2, 5, 6],
                vec![1, 2, 3, 4, 5, 6],
            ),
            (vec![5, 4, 3], vec![3], vec![3, 4, 5]),
            (vec![], vec![1], vec![1]),
            (vec![1], vec![], vec![1]),
            (vec![], vec![], vec![]),
        ];
        for (a, b, golden) in test_cases {
            let a = a.into_iter().map(Option::Some);
            let b = b.into_iter().map(Option::Some);
            let result = super::array_union_generic_sorted(a, b);

            assert_eq!(result, golden);
        }
    }
}
