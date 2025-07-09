use pgrx::{prelude::*, Aggregate, ToAggregateName, Uuid};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, PostgresType, Serialize, Deserialize)]
#[pg_binary_protocol]
pub struct XorAggUuidState {}

impl XorAggUuidState {
    #[inline(always)]
    fn combine(current: Uuid, arg: Uuid) -> Uuid {
        let new = u128::from_ne_bytes(*arg.as_bytes());
        let old = u128::from_ne_bytes(*current.as_bytes());
        Uuid::from_bytes((old ^ new).to_ne_bytes())
    }

    #[inline(always)]
    fn finalize(current: Uuid) -> Uuid {
        current
    }
}
impl ToAggregateName for XorAggUuidState {
    const NAME: &'static str = "bit_xor";
}

#[pg_aggregate]
impl Aggregate<XorAggUuidState> for XorAggUuidState {
    // const INITIAL_CONDITION: Option<&'static str> = Some(r#"{ "agg": 0 }"#);
    type State = Uuid;
    type Args = Uuid;
    type Finalize = Uuid;
    const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);

    #[pgrx(parallel_safe, immutable, strict, create_or_replace)]
    fn state(current: Uuid, arg: Uuid, _fcinfo: pg_sys::FunctionCallInfo) -> Uuid {
        Self::combine(current, arg)
    }

    #[pgrx(parallel_safe, immutable, strict, create_or_replace)]
    fn finalize(
        current: Self::State,
        _direct_args: Self::OrderedSetArgs,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::Finalize {
        Self::finalize(current)
    }

    #[pgrx(parallel_safe, immutable, strict, create_or_replace)]
    fn combine(current: Uuid, other: Uuid, _fcinfo: pg_sys::FunctionCallInfo) -> Uuid {
        Self::combine(current, other)
    }
}
