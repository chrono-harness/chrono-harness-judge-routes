//! Independent examples and universal boundary checks for admitted route operations.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::verified_admission as policy;
use std::collections::BTreeSet;
#[cfg(verus_keep_ghost)]
#[path = "../../judge-routes/src/verified_admission.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::{prelude::*, vpanic as panic};

verus! {

fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn an_operation_is_fresh_only_once_and_every_predecessor_must_have_finished() {
    let mut required = BTreeSet::<u64>::new();
    let mut finished = BTreeSet::<u64>::new();
    expect(policy::predecessors_ready(&required, &finished));
    expect(policy::insert_once(&mut required, 3));
    expect(policy::insert_once(&mut required, 8));
    expect(!policy::insert_once(&mut required, 3));
    expect(!policy::predecessors_ready(&required, &finished));
    finished.insert(8);
    expect(!policy::predecessors_ready(&required, &finished));
    finished.insert(3);
    finished.insert(11);
    expect(policy::predecessors_ready(&required, &finished));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn removal_requires_downstream_or_a_replacement_and_propagates_the_original_error() {
    let mut selected = BTreeSet::<u64>::new();
    let absent: Result<Option<u64>, u64> = Ok(None);
    let answer = policy::apply_retirement(&mut selected, absent, false);
    expect(matches!(answer, Ok(false)));
    let answer = policy::apply_retirement(&mut selected, absent, true);
    expect(matches!(answer, Ok(true)));
    let answer = policy::apply_retirement(&mut selected, Ok::<_,u64>(Some(7)), false);
    expect(matches!(answer, Ok(true)));
    expect(!policy::insert_once(&mut selected, 7));
    let answer = policy::apply_retirement(&mut selected, Err(9u64), true);
    expect(matches!(answer, Err(9)));
}

fn check_target_identity(plan: String, operation: String, other: String)
    requires plan@ != other@, operation@ != other@,
{
    expect(policy::receipt_target_matches(&plan, &plan, &operation, &operation));
    expect(!policy::receipt_target_matches(&plan, &other, &operation, &operation));
    expect(!policy::receipt_target_matches(&plan, &plan, &operation, &other));
    expect(policy::stream_matches(&operation, &operation, true));
    expect(!policy::stream_matches(&operation, &other, true));
    expect(!policy::stream_matches(&operation, &operation, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_sealed_receipt_keeps_run_identity_and_checks_each_stream_at_the_limit() {
    proof {
        reveal_strlit("plan"); reveal_strlit("op"); reveal_strlit("");
        assert("plan"@.len() != ""@.len());
        assert("op"@.len() != ""@.len());
    }
    check_target_identity("plan".to_string(), "op".to_string(), "".to_string());
    expect(policy::output_sizes_fit(4, 4, 4));
    expect(!policy::output_sizes_fit(5, 0, 4));
    expect(!policy::output_sizes_fit(0, 5, 4));
}

}
