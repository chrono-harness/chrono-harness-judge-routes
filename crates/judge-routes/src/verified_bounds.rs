//! Bounds used in both declared operations and collected execution plans.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub fn valid_execution_bounds(seconds: u64, bytes: usize) -> (valid: bool)
    ensures valid == (seconds > 0 && 1 <= bytes <= 67_108_864),
{
    seconds > 0 && bytes >= 1 && bytes <= 67_108_864
}

}
