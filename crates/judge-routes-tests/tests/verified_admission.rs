//! Independent examples and universal boundary checks for admitted route operations.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::verified_admission as policy;
use std::collections::{BTreeMap, BTreeSet};
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
fn assignment_rejects_unknown_and_repeated_plans_without_changing_membership() {
    let mut plans = BTreeSet::<u64>::new();
    let mut assigned = BTreeSet::<u64>::new();
    plans.insert(3);
    expect(matches!(policy::assign_plan(&plans, &mut assigned, 7), policy::PlanAssignment::Unknown));
    expect(!policy::predecessors_ready(&plans, &assigned));
    expect(matches!(policy::assign_plan(&plans, &mut assigned, 3), policy::PlanAssignment::Accepted));
    expect(matches!(policy::assign_plan(&plans, &mut assigned, 3), policy::PlanAssignment::Duplicate));
    expect(policy::predecessors_ready(&plans, &assigned));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn shared_rows_bind_the_entire_set_of_registered_units() {
    let mut known = BTreeMap::<u64, u64>::new();
    known.insert(1, 0);
    known.insert(2, 0);
    let mut actual = BTreeSet::<u64>::new();
    actual.insert(1);
    expect(policy::shared_operation_matches(&actual, None));
    expect(!policy::valid_shared_row(&actual, 1, &known));
    actual.insert(2);
    expect(!policy::shared_operation_matches(&actual, None));
    expect(policy::valid_shared_row(&actual, 2, &known));
    expect(!policy::valid_shared_row(&actual, 3, &known));
    let mut declared = BTreeSet::<u64>::new();
    declared.insert(1);
    declared.insert(2);
    assert(actual@ =~= declared@);
    expect(policy::shared_operation_matches(&actual, Some(&declared)));
    declared.insert(3);
    assert(declared@.contains(3) && !actual@.contains(3));
    expect(!policy::shared_operation_matches(&actual, Some(&declared)));
    expect(!policy::valid_shared_row(&declared, 3, &known));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn an_operation_is_fresh_only_once_and_every_predecessor_must_have_finished() {
    let mut required = BTreeSet::<u64>::new();
    let mut finished = BTreeSet::<u64>::new();
    expect(policy::predecessors_ready(&required, &finished));
    expect(policy::insert_once(&mut required, 3));
    expect(policy::insert_once(&mut required, 8));
    expect(!policy::insert_once(&mut required, 3));
    assert(required@.contains(3) && !finished@.contains(3));
    expect(!policy::predecessors_ready(&required, &finished));
    finished.insert(8);
    assert(required@.contains(3) && !finished@.contains(3));
    expect(!policy::predecessors_ready(&required, &finished));
    finished.insert(3);
    finished.insert(11);
    expect(policy::predecessors_ready(&required, &finished));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registration_decisions_cover_the_exact_retirement_query() {
    let mut request = BTreeSet::<u64>::new();
    let mut decisions = BTreeSet::<u64>::new();
    let id = "collect".to_string();
    expect(policy::retirement_reply_matches(None, None, &request, &decisions));
    expect(!policy::retirement_reply_matches(Some(&id), None, &request, &decisions));
    expect(!policy::retirement_reply_matches(None, Some(&id), &request, &decisions));
    request.insert(2);
    assert(request@.contains(2) && !decisions@.contains(2));
    expect(!policy::retirement_reply_matches(Some(&id), Some(&id), &request, &decisions));
    decisions.insert(3);
    assert(request@.contains(2) && !decisions@.contains(2));
    expect(!policy::retirement_reply_matches(Some(&id), Some(&id), &request, &decisions));
    decisions.insert(2);
    assert(!request@.contains(3) && decisions@.contains(3));
    expect(!policy::retirement_reply_matches(Some(&id), Some(&id), &request, &decisions));
    request.insert(3);
    assert(request@ =~= decisions@);
    expect(policy::retirement_reply_matches(Some(&id), Some(&id), &request, &decisions));
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn tool_probe_requires_both_streams_before_reading_the_environment() {
    // The error must survive only when all earlier observations succeed.
    let good = || -> (v: bool) ensures v { true };
    let bad = || -> (v: bool) ensures !v { false };
    let matching_env = || -> (r: Result<bool, String>) ensures r == Ok::<bool, String>(true) { Ok(true) };
    let other_env = || -> (r: Result<bool, String>) ensures r == Ok::<bool, String>(false) { Ok(false) };
    let read_error = || -> (r: Result<bool, String>)
        ensures match r { Err(e) => e@ == "environment encoding"@, Ok(_) => false }
    { Err("environment encoding".to_string()) };
    expect(matches!(policy::probe_matches(1, false, good, good, read_error), Ok(false)));
    expect(matches!(policy::probe_matches(0, true, good, good, read_error), Ok(false)));
    expect(matches!(policy::probe_matches(0, false, bad, good, read_error), Ok(false)));
    expect(matches!(policy::probe_matches(0, false, good, bad, read_error), Ok(false)));
    expect(matches!(policy::probe_matches(0, false, good, good, other_env), Ok(false)));
    expect(matches!(policy::probe_matches(0, false, good, good, matching_env), Ok(true)));
    match policy::probe_matches(0, false, good, good, read_error) {
        Err(error) => {
            assert(error@ == "environment encoding"@);
            expect(error == "environment encoding".to_string());
        },
        Ok(_) => panic!("environment error lost"),
    }
}

}
