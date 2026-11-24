//! Independent lower and upper boundaries of an executable route operation.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::verified_bounds as policy;
#[cfg(verus_keep_ghost)]
#[path = "../../judge-routes/src/verified_bounds.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::{prelude::*, vpanic as panic};

verus! {

fn expect(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn limits_of_executable_operations() {
    expect(!policy::valid_execution_bounds(0, 1));
    expect(!policy::valid_execution_bounds(1, 0));
    expect(policy::valid_execution_bounds(1, 1));
    expect(policy::valid_execution_bounds(u64::MAX, 64 * 1024 * 1024));
    expect(!policy::valid_execution_bounds(1, 64 * 1024 * 1024 + 1));
}

}
