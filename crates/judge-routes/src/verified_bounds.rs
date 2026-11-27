//! Bounds used in both declared operations and collected execution plans.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "declarations.rs"]
pub mod timeout_bounds;
#[cfg(not(verus_keep_ghost))]
pub mod timeout_bounds {
    pub use crate::declarations::decimal_timeout;
}

#[cfg(verus_keep_ghost)]
#[path = "evidence.rs"]
pub mod observed_timeouts;
#[cfg(not(verus_keep_ghost))]
pub use crate::evidence as observed_timeouts;

verus! {

pub fn valid_execution_bounds(seconds: u64, bytes: usize) -> (valid: bool)
    ensures valid == (seconds > 0 && 1 <= bytes <= 67_108_864),
{
    seconds > 0 && bytes >= 1 && bytes <= 67_108_864
}

}
