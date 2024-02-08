use std::cmp::Ordering;

use bytes::{BufMut, Bytes, BytesMut};
use fasthash::farm;
use itertools::Itertools;
use pgrx::{prelude::*, Aggregate, PostgresType, Uuid, VariadicArray};
use serde::{Deserialize, Serialize};

pgrx::pg_module_magic!();

static ZERO_BYTE_ARRAY: [u8; 1] = [0];

#[derive(Clone, Default, PostgresType, Serialize, Deserialize)]
pub struct SeahashState {
    data: Vec<String>,
}

impl SeahashState {
    #[inline(always)]
    fn state(
        mut current: <Self as Aggregate>::State,
        arg: <Self as Aggregate>::Args,
    ) -> <Self as Aggregate>::State {
        // explicitly panic if NULL
        current.data.push(arg.expect("NULL value given!"));
        current
    }

    #[inline(always)]
    fn finalize(mut current: <Self as Aggregate>::State) -> <Self as Aggregate>::Finalize {
        let iter = current.data.into_iter();
        current.data = vec!();
        seahash_fingerprint(id_iter_to_bytes(iter))
    }
}

#[pg_aggregate]
impl Aggregate for SeahashState {
    const NAME: &'static str = "seahash_agg";
    const INITIAL_CONDITION: Option<&'static str> = Some(r#"{ "data": [] }"#);
    type State = Self;
    type Args = pgrx::name!(value, Option<String>);
    type Finalize = i64;

    #[pgrx(parallel_safe, immutable)]
    fn state(
        current: <Self as Aggregate>::State,
        arg: <Self as Aggregate>::Args,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> <Self as Aggregate>::State {
        Self::state(current, arg)
    }

    #[pgrx(parallel_safe, immutable)]
    fn finalize(
        current: <Self as Aggregate>::State,
        _direct_args: Self::OrderedSetArgs,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> <Self as Aggregate>::Finalize {
        Self::finalize(current)
    }
}

#[derive(Clone, Default, PostgresType, Serialize, Deserialize)]
pub struct FarmhashState {
    data: Vec<String>,
}

impl FarmhashState {
    #[inline(always)]
    fn state(
        mut current: <Self as Aggregate>::State,
        arg: <Self as Aggregate>::Args,
    ) -> <Self as Aggregate>::State {
        // explicitly panic if NULL
        current.data.push(arg.expect("NULL value given!"));
        current
    }

    #[inline(always)]
    fn finalize(mut current: <Self as Aggregate>::State) -> <Self as Aggregate>::Finalize {
        let iter = current.data.into_iter();
        current.data = vec!();
        farmhash_fingerprint(id_iter_to_bytes(iter))
    }
}

#[pg_aggregate]
impl Aggregate for FarmhashState {
    const NAME: &'static str = "farmhash_agg";
    const INITIAL_CONDITION: Option<&'static str> = Some(r#"{ "data": [] }"#);
    type State = Self;
    type Args = pgrx::name!(value, Option<String>);
    type Finalize = Uuid;

    #[pgrx(parallel_safe, immutable)]
    fn state(
        current: <Self as Aggregate>::State,
        arg: <Self as Aggregate>::Args,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> <Self as Aggregate>::State {
        Self::state(current, arg)
    }

    #[pgrx(parallel_safe, immutable)]
    fn finalize(
        current: <Self as Aggregate>::State,
        _direct_args: Self::OrderedSetArgs,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> <Self as Aggregate>::Finalize {
        Self::finalize(current)
    }
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of strings into a md5 using md5 with _ as separator.
pub fn id_underscore_md5(a: VariadicArray<String>) -> Uuid {
    let res = a.iter_deny_null().collect::<Vec<String>>().join("_");
    let digest = md5::compute(res);
    Uuid::from_bytes(digest.0)
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of strings into a Uuid using farmhash's fingerprint128
pub fn id_farmhash(a: VariadicArray<String>) -> Uuid {
    farmhash_fingerprint(ids_to_bytes(a))
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of strings into a bigint using seahash
pub fn id_seahash(a: VariadicArray<String>) -> i64 {
    seahash_fingerprint(ids_to_bytes(a))
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of pairs key, value of strings into a Uuid using farmhash's fingerprint128
pub fn checksum_farmhash(a: VariadicArray<String>) -> Uuid {
    assert!(a.len() % 2 == 0);
    let b = normalized_pairs_bytes(a.iter(), filter_and_join_tuple_keep_null_values);
    farmhash_fingerprint(b)
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of pairs key, value of strings into a Uuid using farmhash's fingerprint128, skipping NULL values
pub fn checksum_farmhash_extendable(a: VariadicArray<String>) -> Uuid {
    assert!(a.len() % 2 == 0);
    let b = normalized_pairs_bytes(a.iter(), filter_and_join_tuple);
    farmhash_fingerprint(b)
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of pairs key, value of strings into a Uuid using seahash.
pub fn checksum_seahash(a: VariadicArray<String>) -> i64 {
    assert!(a.len() % 2 == 0);
    let b = normalized_pairs_bytes(a.iter(), filter_and_join_tuple_keep_null_values);
    seahash_fingerprint(b)
}

#[pg_extern(strict, immutable, parallel_safe)]
/// Hash a variadic array of pairs key, value of strings into a Uuid using seahash, skipping NULL values.
pub fn checksum_seahash_extendable(a: VariadicArray<String>) -> i64 {
    assert!(a.len() % 2 == 0);
    let b = normalized_pairs_bytes(a.iter(), filter_and_join_tuple);
    seahash_fingerprint(b)
}

#[inline]
fn seahash_fingerprint(a: Bytes) -> i64 {
    let digest = seahash::hash(&a);

    // reversible u64 -> i64 by bytecasting
    i64::from_ne_bytes(digest.to_ne_bytes())
}

#[inline]
fn farmhash_fingerprint(a: Bytes) -> Uuid {
    let digest = farm::fingerprint128(a);
    Uuid::from_bytes(digest.to_le_bytes())
}

#[inline]
fn ids_to_bytes(a: VariadicArray<String>) -> Bytes {
    id_iter_to_bytes(a.iter_deny_null())
}

#[inline]
fn id_iter_to_bytes(a: impl Iterator<Item = String>) -> Bytes {
    #[allow(unstable_name_collisions)] // silence warning about intersperse
    let vec: Vec<Bytes> = a
        .map(Bytes::from)
        .intersperse(Bytes::from_static(&ZERO_BYTE_ARRAY))
        .collect();
    vec.concat().into()
}

#[inline]
fn join_tuple(a: Bytes, b: Bytes) -> Bytes {
    let mut buf = BytesMut::with_capacity(a.len() + b.len() + 1);
    buf.put(a);
    buf.put_u8(0);
    buf.put(b);
    buf.freeze()
}

#[inline]
fn filter_and_join_tuple((a, b): (Option<Bytes>, Option<Bytes>)) -> Option<Bytes> {
    if let (Some(a), Some(b)) = (a, b) {
        Some(join_tuple(a, b))
    } else {
        None
    }
}

#[inline]
fn filter_and_join_tuple_keep_null_values((a, b): (Option<Bytes>, Option<Bytes>)) -> Option<Bytes> {
    match (a, b) {
        (Some(a), Some(b)) => Some(join_tuple(a, b)),
        (Some(a), _) => Some(join_tuple(a, Bytes::new())),
        _ => None,
    }
}

#[inline]
fn normalized_pairs_bytes<T: Iterator, F>(a: T, f: F) -> Bytes
where
    T: Iterator<Item = Option<String>>,
    F: FnMut((Option<Bytes>, Option<Bytes>)) -> Option<Bytes>,
{
    let mut vec: Vec<_> = a.map(|e| e.map(Bytes::from)).tuples().collect();

    vec.sort_unstable_by(cmp_option_tuple);

    // filter out same key-value tuples
    vec.dedup();

    #[allow(unstable_name_collisions)] // silence warning about intersperse
    let vec: Vec<_> = vec
        .into_iter()
        .flat_map(f)
        .intersperse(Bytes::from_static(&ZERO_BYTE_ARRAY))
        .collect();
    vec.concat().into()
}

#[inline]
fn cmp_option_tuple<T, T_>(a: &(Option<T>, Option<T_>), b: &(Option<T>, Option<T_>)) -> Ordering
where
    T: Ord,
{
    match (a, b) {
        ((Some(a), _), (Some(b), _)) => a.cmp(b),
        ((Some(_), _), _) => Ordering::Greater,
        (_, (Some(_), _)) => Ordering::Less,
        (_, _) => Ordering::Equal,
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use bytes::Bytes;
    use fasthash::farm;
    use pgrx::prelude::*;
    use pgrx::Uuid;

    static CHECKSUM_GOLDEN_TABLE: [(&str, u128, u64, u128, u64); 7] = [
        (
            "'b','1','a','2','c','3'",
            200872945134416140889070688363161139169,
            2409718014940744358,
            200872945134416140889070688363161139169,
            2409718014940744358,
        ),
        (
            "'c','1','b','2','a','3'",
            159497434488907202746785405570285430728,
            13904897432444956006,
            159497434488907202746785405570285430728,
            13904897432444956006,
        ),
        (
            "'a','1','b','2','c','3'",
            63991421267038837894546220157490372611,
            2987942282093369026,
            63991421267038837894546220157490372611,
            2987942282093369026,
        ),
        (
            "'c','3','b','2','a','1'",
            63991421267038837894546220157490372611,
            2987942282093369026,
            63991421267038837894546220157490372611,
            2987942282093369026,
        ),
        (
            "'d',NULL,'b','1','a','2','c','3'",
            200872945134416140889070688363161139169,
            2409718014940744358,
            81995647272375524016145587428893207932,
            1483524512128736869,
        ),
        (
            "'b','1','e,', NULL, 'a','2','c','3','d',NULL",
            200872945134416140889070688363161139169,
            2409718014940744358,
            25535654360990964780919159173614512903,
            2918039635165515565,
        ),
        (
            "'a','1','d',NULL,'b','2','c','3'",
            63991421267038837894546220157490372611,
            2987942282093369026,
            55229287721383068868671312639375865447,
            13930034059081198257,
        ),
    ];

    static ID_GOLDEN_TABLE: [(Bytes, &str, u128, u64); 4] = [
        (
            Bytes::from_static(b"a\0b\0c"),
            "'a','b','c'",
            185254626185375829619130502206294491400,
            14302911629075895706,
        ),
        (
            Bytes::from_static(b"a\0bc"),
            "'a','bc'",
            209923663634918632141264334831867734826,
            11633404322457790885,
        ),
        (
            Bytes::from_static(b"ab\0c"),
            "'ab','c'",
            48977107076008525069529680716651780466,
            15405044104555301515,
        ),
        (
            Bytes::from_static(b"ab\0c\0d"),
            "'ab','c','d'",
            236038958675336168148183549336891112308,
            15729725979387502697,
        ),
    ];

    #[test]
    fn test_normalized_pairs_keeping_nulls() {
        let table = vec![
            (
                vec!["a", "v", "b", "v2"],
                Bytes::from_static(b"a\0v\0b\0v2"),
            ),
            (
                vec!["b", "v2", "a", "v"],
                Bytes::from_static(b"a\0v\0b\0v2"),
            ),
            (
                vec!["c", "v3", "a", "v", "b", "v2"],
                Bytes::from_static(b"a\0v\0b\0v2\0c\0v3"),
            ),
            (
                vec!["b", "v2", "c", "v3", "a", "v"],
                Bytes::from_static(b"a\0v\0b\0v2\0c\0v3"),
            ),
        ];
        for (arr, bytes) in table.iter() {
            let arr_iter = arr.iter().map(|s| s.to_string()).map(Option::Some);
            let result = crate::normalized_pairs_bytes(
                arr_iter,
                crate::filter_and_join_tuple_keep_null_values,
            );
            assert_eq!(result, bytes, "using {:?}", arr);
        }

        let table = vec![
            (
                vec![Some("c"), None, Some("a"), None, Some("b"), Some("v2")],
                Bytes::from_static(b"a\0\0b\0v2\0c\0"),
            ),
            (
                vec![
                    Some("c"),
                    Some("v3"),
                    Some("a"),
                    None,
                    Some("b"),
                    Some("v2"),
                ],
                Bytes::from_static(b"a\0\0b\0v2\0c\0v3"),
            ),
            (
                vec![
                    Some("d"),
                    None,
                    Some("b"),
                    Some("1"),
                    Some("a"),
                    None,
                    Some("c"),
                    Some("3"),
                ],
                Bytes::from_static(b"a\0\0b\01\0c\03\0d\0"),
            ),
        ];
        for (arr, bytes) in table.iter() {
            let arr_iter = arr.iter().map(|s| s.map(|s| s.to_string()));
            let result = crate::normalized_pairs_bytes(
                arr_iter,
                crate::filter_and_join_tuple_keep_null_values,
            );
            assert_eq!(result, bytes, "using {:?}", arr);
        }
    }

    #[test]
    fn test_normalized_pairs() {
        let table = vec![
            (
                vec!["a", "v", "b", "v2"],
                Bytes::from_static(b"a\0v\0b\0v2"),
            ),
            (
                vec!["b", "v2", "a", "v"],
                Bytes::from_static(b"a\0v\0b\0v2"),
            ),
            (
                vec!["c", "v3", "a", "v", "b", "v2"],
                Bytes::from_static(b"a\0v\0b\0v2\0c\0v3"),
            ),
            (
                vec!["b", "v2", "c", "v3", "a", "v"],
                Bytes::from_static(b"a\0v\0b\0v2\0c\0v3"),
            ),
        ];
        for (arr, bytes) in table.iter() {
            let arr_iter = arr.iter().map(|s| s.to_string()).map(Option::Some);
            let result = crate::normalized_pairs_bytes(arr_iter, crate::filter_and_join_tuple);
            assert_eq!(result, bytes, "using {:?}", arr);
        }

        let table = vec![
            (
                vec![Some("c"), Some("v3"), Some("a"), None, Some("b"), None],
                Bytes::from_static(b"c\0v3"),
            ),
            (
                vec![
                    Some("c"),
                    Some("v3"),
                    Some("a"),
                    None,
                    Some("b"),
                    Some("v2"),
                ],
                Bytes::from_static(b"b\0v2\0c\0v3"),
            ),
            (
                vec![
                    Some("d"),
                    None,
                    Some("b"),
                    Some("1"),
                    Some("a"),
                    None,
                    Some("c"),
                    Some("3"),
                ],
                Bytes::from_static(b"b\01\0c\03"),
            ),
        ];
        for (arr, bytes) in table.iter() {
            let arr_iter = arr.iter().map(|s| s.map(|s| s.to_string()));
            let result = crate::normalized_pairs_bytes(arr_iter, crate::filter_and_join_tuple);
            assert_eq!(result, bytes, "using {:?}", arr);
        }
    }

    #[test]
    fn test_farmhash() {
        for (bytes, params, golden, _) in ID_GOLDEN_TABLE.iter() {
            let result = farm::fingerprint128(bytes);
            assert_eq!(result, *golden, "using {:?}", params);
        }
    }

    #[test]
    fn test_seahash() {
        for (bytes, params, _, golden) in ID_GOLDEN_TABLE.iter() {
            let result = seahash::hash(bytes);
            assert_eq!(result, *golden, "using {}", params);
        }
    }

    #[pg_test]
    fn pg_test_id_underscore_md5() {
        let result = Spi::get_one::<Uuid>("SELECT id_underscore_md5('1','2','3');")
            .expect("didn't get SPI result");
        let golden =
            Spi::get_one::<Uuid>("SELECT md5('1_2_3')::uuid;").expect("didn't get SPI result");
        assert_eq!(result, golden);
    }

    #[pg_test]
    fn pg_test_id_farmhash() {
        for (_, params, golden, _) in ID_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<Uuid>(&format!("SELECT id_farmhash({});", params))
                .expect("didn't get SPI result")
                .expect("got None");
            let golden_uuid: Uuid = Uuid::from_bytes(golden.to_le_bytes());
            assert_eq!(result, golden_uuid, "using {}", params);
        }
    }

    #[pg_test]
    fn pg_test_id_seahash() {
        for (_, params, _, golden) in ID_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<i64>(&format!("SELECT id_seahash({});", params))
                .expect("didn't get SPI result")
                .expect("got None");
            let result_u64 = u64::from_ne_bytes(result.to_ne_bytes());
            assert_eq!(result_u64, *golden, "using {}", params);
        }
    }

    fn farmhash_agg(data: Vec<Option<String>>) -> Uuid {
        let mut state = crate::FarmhashState::default();
        for a in data.iter() {
            state = crate::FarmhashState::state(state, a.clone());
        }
        crate::FarmhashState::finalize(state)
    }

    #[test]
    #[should_panic]
    fn test_checksum_farmhash_agg_fails_with_nulls() {
        let v = vec!(Some('d'.to_string()),None,Some('b'.to_string()));
        let _ = farmhash_agg(v);
    }

    #[pg_test]
    fn pg_test_farmhash_agg() {
        for (_, params, golden, _) in ID_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<Uuid>(&format!(
                "SELECT farmhash_agg(v) FROM UNNEST(ARRAY[{}]) v;",
                params
            ))
            .expect("didn't get SPI result")
            .expect("got None");

            assert_eq!(
                u128::from_le_bytes(*result.as_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }


    #[pg_test]
    #[should_panic]
    fn pg_test_farmhash_agg_fail_with_null() {
        let _ = Spi::run(&format!(
            "SELECT farmhash_agg(v) FROM UNNEST(ARRAY[{}]) v;",
             "'a','d',NULL,'b','c'"
        ));
    }

    fn seahash_agg(data: Vec<Option<String>>) -> i64 {
        let mut state = crate::SeahashState::default();
        for a in data.iter() {
            state = crate::SeahashState::state(state, a.clone());
        }
        crate::SeahashState::finalize(state)
    }

    #[test]
    #[should_panic]
    fn test_checksum_seahash_agg_fails_with_nulls() {
        let v = vec!(Some('d'.to_string()),None,Some('b'.to_string()));
        let _ = seahash_agg(v);
    }

    #[pg_test]
    fn pg_test_seahash_agg() {
        for (_, params, _, golden) in ID_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<i64>(&format!(
                "SELECT seahash_agg(v) FROM UNNEST(ARRAY[{}]) v;",
                params
            ))
            .expect("didn't get SPI result")
            .expect("got None");

            assert_eq!(
                u64::from_ne_bytes(result.to_ne_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }


    #[pg_test]
    #[should_panic]
    fn pg_test_seahash_agg_fail_with_null() {
        let _ = Spi::run(&format!(
            "SELECT seahash_agg(v) FROM UNNEST(ARRAY[{}]) v;",
             "'a','d',NULL,'b','c'"
        ));
    }

    #[pg_test]
    fn pg_test_checksum_farmhash() {
        for (params, _, _, golden, _) in CHECKSUM_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<Uuid>(&format!("SELECT checksum_farmhash({});", params))
                .expect("didn't get SPI result")
                .expect("got None");

            assert_eq!(
                u128::from_le_bytes(*result.as_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }

    #[pg_test]
    fn pg_test_checksum_seahash() {
        for (params, _, _, _, golden) in CHECKSUM_GOLDEN_TABLE.iter() {
            let result = Spi::get_one::<i64>(&format!("SELECT checksum_seahash({});", params))
                .expect("didn't get SPI result")
                .expect("got None");

            assert_eq!(
                u64::from_ne_bytes(result.to_ne_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }
    #[pg_test]
    fn pg_test_checksum_farmhash_extendable() {
        for (params, golden, _, _, _) in CHECKSUM_GOLDEN_TABLE.iter() {
            let result =
                Spi::get_one::<Uuid>(&format!("SELECT checksum_farmhash_extendable({});", params))
                    .expect("didn't get SPI result")
                    .expect("got None");

            assert_eq!(
                u128::from_le_bytes(*result.as_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }

    #[pg_test]
    fn pg_test_checksum_seahash_extendable() {
        for (params, _, golden, _, _) in CHECKSUM_GOLDEN_TABLE.iter() {
            let result =
                Spi::get_one::<i64>(&format!("SELECT checksum_seahash_extendable({});", params))
                    .expect("didn't get SPI result")
                    .expect("got None");

            assert_eq!(
                u64::from_ne_bytes(result.to_ne_bytes()),
                *golden,
                "using {}",
                params
            );
        }
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
