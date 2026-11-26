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

#[path = "../examples/release_observation.rs"]
mod release_observation;

verus! {

#[cfg(verus_keep_ghost)]
fn documented_commands_and_arity(name: &str, extra: bool) {
    use policy::cli::{command, Query, Command};
    reveal_strlit("--version");
    reveal_strlit("evaluate-routes-v1");
    reveal_strlit("global-selection");
    reveal_strlit("collected-selection-v1");
    reveal_strlit("check-plan-receipts-v1");
    reveal_strlit("check-operation-receipt-v1");
    reveal_strlit("plan-context-v1");
    reveal_strlit("collection-plan-v1");
    reveal_strlit("full-operations-v1");
    reveal_strlit("source-plan-v1");
    reveal_strlit("check-unit-assignments-v1");
    reveal_strlit("unit-declarations-v1");
    reveal_strlit("retained-tool-v1");
    reveal_strlit("check-plan-v1");
    reveal_strlit("dispatch-plan-v1");
    reveal_strlit("check-scoped-plan-v1");
    reveal_strlit("scoped-operations-v1");
    reveal_strlit("prepare-scoped-plan-v1");
    let result = command(name, extra);
    proof {
    if name@ == "--version"@ {
        assert((result is Version) == !extra);
        assert((result is Judge) == extra);
    } else if name@ == "evaluate-routes-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::Evaluate) });
    } else if name@ == "global-selection"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::GlobalSelection) });
    } else if name@ == "collected-selection-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::CollectedSelection) });
    } else if name@ == "check-plan-receipts-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::PlanReceipts) });
    } else if name@ == "check-operation-receipt-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::OperationReceipt) });
    } else if name@ == "plan-context-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::PlanContext) });
    } else if name@ == "collection-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::CollectionPlan) });
    } else if name@ == "full-operations-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::FullOperations) });
    } else if name@ == "source-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::SourcePlan) });
    } else if name@ == "check-unit-assignments-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::UnitAssignments) });
    } else if name@ == "unit-declarations-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::UnitDeclarations) });
    } else if name@ == "retained-tool-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::RetainedTool) });
    } else if name@ == "check-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::CheckPlan) });
    } else if name@ == "dispatch-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::DispatchPlan) });
    } else if name@ == "check-scoped-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::ScopedPlan) });
    } else if name@ == "scoped-operations-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::ScopedOperations) });
    } else if name@ == "prepare-scoped-plan-v1"@ {
        assert(if extra { result is Usage } else { result matches Command::Query(Query::PrepareScopedPlan) });
    } else {
        assert(result is Judge);
    }
    }
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn release_accepts_the_expected_observations_only() {
    let accepted = release_observation::accepts(2, 4, Some(1), true);
    assert(accepted);
    let refused_1 = release_observation::accepts(0, 4, Some(1), true);
    assert(!refused_1);
    let refused_2 = release_observation::accepts(2, 0, Some(1), true);
    assert(!refused_2);
    let refused_3 = release_observation::accepts(2, 4, None, true);
    assert(!refused_3);
    let refused_4 = release_observation::accepts(2, 4, Some(2), true);
    assert(!refused_4);
    let refused_5 = release_observation::accepts(2, 4, Some(1), false);
    assert(!refused_5);
}


/// The same real stdout and stderr may occur with either request-write outcome.
fn required_runner_response(success: bool, raw: Vec<u8>, err: Vec<u8>, sent: Result<(), String>) {
    let ghost out = raw@;
    let ghost diagnostic = err@;
    let ghost wrote = sent;
    match policy::runner_response(success, raw, err, sent) {
        Ok(bytes) => { proof { assert(success && wrote is Ok && bytes@ == out); } }
        Err(policy::RunnerError::ExitStatus) => {
            proof { assert(!success && diagnostic.len() == 0); }
        }
        Err(policy::RunnerError::Diagnostic(bytes)) => {
            proof { assert(!success && diagnostic.len() > 0 && bytes@ == diagnostic); }
        }
        Err(policy::RunnerError::Request(message)) => {
            proof { assert(success && wrote == Err::<(), String>(message)); }
        }
    }
}


fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn dispatch_ranks_admit_distinct_claims_and_place_unranked_operations_last() {
    let mut ranks = BTreeMap::<u8, usize>::new();
    let good = [97u8, 98u8];
    let bad = [97u8, 0u8, 98u8];
    let empty: [u8; 0] = [];
    expect(!policy::record_priority(&mut ranks, 1, &empty, true, 0));
    expect(!policy::record_priority(&mut ranks, 1, &bad, true, 0));
    expect(!policy::record_priority(&mut ranks, 1, &good, false, 0));
    expect(policy::record_priority(&mut ranks, 2, &good, true, 0));
    expect(policy::record_priority(&mut ranks, 1, &good, true, 1));
    expect(policy::priority_key(&ranks, &2) == 0);
    expect(policy::priority_key(&ranks, &1) == 1);
    expect(policy::priority_key(&ranks, &3) == usize::MAX);
    expect(!policy::record_priority(&mut ranks, 2, &good, true, 2));
}

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

// Independent of the wire decoder and the implementation's comparison order.
fn receipt_contract(method: &String, recorded: &String, other: &String, error: &String)
    requires method@ != other@, recorded@ != other@,
{
    let correct = || -> (d: Result<String,String>) ensures d == Ok::<String,String>(*recorded) { Ok(recorded.clone()) };
    let failed = || -> (d: Result<String,String>) ensures d == Err::<String,String>(*error) { Err(error.clone()) };
    expect(matches!(policy::receipt_seal_matches(method, Ok(method.clone()), recorded, correct), Ok(true)));
    expect(matches!(policy::receipt_seal_matches(method, Ok(other.clone()), recorded, failed), Ok(false)));
    expect(matches!(policy::receipt_seal_matches(method, Ok(method.clone()), other, correct), Ok(false)));
    let first = policy::receipt_seal_matches(method, Err(other.clone()), recorded, failed);
    proof { assert(first == Err::<bool,String>(*other)); }
    let second = policy::receipt_seal_matches(method, Ok(method.clone()), recorded, failed);
    proof { assert(second == Err::<bool,String>(*error)); }
}

fn context_contract(cwd: bool, executable: bool, seen: &String, wanted: &String,
    same_environment: bool, environment: &String, calculated: &String, stdin: &String, empty: &String, error: &String)
{
    let tool = policy::receipt_tool_matches(cwd && executable, seen, wanted);
    proof { assert(tool == (cwd && executable && seen@ == wanted@)); }
    let hash = || -> (r: Result<String,String>) ensures r == Ok::<String,String>(*calculated) { Ok(calculated.clone()) };
    let input = || -> (s: String) ensures s@ == empty@ { empty.clone() };
    let checked = policy::receipt_input_matches(same_environment, environment, hash, stdin, input);
    proof { assert(checked == Ok::<bool,String>(same_environment && environment@ == calculated@ && stdin@ == empty@)); }
    let failure = || -> (r: Result<String,String>) ensures r == Err::<String,String>(*error) { Err(error.clone()) };
    let checked = policy::receipt_input_matches(same_environment, environment, failure, stdin, input);
    proof { assert(checked == (if same_environment { Err::<bool,String>(*error) } else { Ok::<bool,String>(false) })); }
}

fn compiler_dispatch(contract: bool, dependency: bool, artifact: bool) {
    let selected = policy::compiler_check_required(contract, dependency, artifact);
    proof { assert(!selected == (!contract && !dependency && !artifact)); }
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
