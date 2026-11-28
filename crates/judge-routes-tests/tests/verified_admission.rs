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

#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::source_plan::{Reply, bind_coverage_reply};
#[cfg(verus_keep_ghost)]
use policy::source_plan::{Reply, bind_coverage_reply};

#[path = "../../../.chrono-harness/ci/roadmap.rs"]
mod roadmap;

verus! {

#[cfg(verus_keep_ghost)]
fn scoped_run_requires_an_original_run_and_a_derived_directory(
    report: &str, digest: &str, identity: &u64,
) {
    let original = policy::plan_query::scoped_run(false, Some(identity));
    assert(original == Ok::<&u64, &'static str>(identity));
    let absent = policy::plan_query::scoped_run::<u64>(false, None);
    assert(absent matches Err(e) && e@ == "E_PLAN_BINDING: missing scoped run identity"@);
    let supplied = policy::plan_query::scoped_run::<u64>(true, None);
    let both = policy::plan_query::scoped_run(true, Some(identity));
    assert(supplied matches Err(e) && e@ == "E_PLAN_BINDING: operation_state is derived from the report"@);
    assert(both matches Err(e) && e@ == "E_PLAN_BINDING: operation_state is derived from the report"@);
    let state = policy::plan_query::operation_directory(report, digest);
    assert(state@ == report@ + ".operations-"@ + digest@);
}

fn check_coverage_reply(previous: Option<&str>, candidate: &str, old: Option<String>,
    reported: String, facts: u8, error: Option<String>) {
    let previous_equal = match (previous, &old) {
        (Some(wanted), Some(seen)) => wanted.to_string() == *seen,
        _ => previous.is_none() && old.is_none(),
    };
    let candidate_equal = candidate.to_string() == reported;
    let reply = Reply { previous: old, candidate: reported, git_facts: facts, error };
    let result = bind_coverage_reply(previous, candidate, reply);
    match result {
        Ok(kept) => {
            expect(candidate_equal && previous_equal);
            expect(kept.git_facts == facts);
            assert(kept.previous == old && kept.candidate == reported && kept.error == error);
        },
        Err(reason) => {
            expect(!candidate_equal || !previous_equal);
            expect(reason.to_string() == "E_GIT_FACTS: source coverage endpoint identity mismatch".to_string());
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn coverage_replies_keep_matching_endpoints_and_reject_replacements_or_omissions() {
    check_coverage_reply(None, "head", None, "head".to_string(), 0, None);
    check_coverage_reply(Some("base"), "head", Some("base".to_string()), "head".to_string(),
        255, Some("coverage error".to_string()));
    check_coverage_reply(None, "head", Some("base".to_string()), "head".to_string(), 1, None);
    check_coverage_reply(Some("base"), "head", None, "head".to_string(), 2, None);
    check_coverage_reply(Some("base"), "head", Some("wrong".to_string()), "head".to_string(), 3, None);
    check_coverage_reply(Some("base"), "head", Some("base".to_string()), "other".to_string(), 4, None);
    check_coverage_reply(Some("base"), "head", Some("wrong".to_string()), "other".to_string(), 5, None);
}

#[cfg(verus_keep_ghost)]
fn documented_commands_and_arity(name: &str, extra: bool) {
    use policy::cli::{command, Query, Command};
    let result = command(name, extra);
    proof {
        let named = name@ == "evaluate-routes-v1"@
            || name@ == "global-selection"@
            || name@ == "collected-selection-v1"@
            || name@ == "check-plan-receipts-v1"@
            || name@ == "check-operation-receipt-v1"@
            || name@ == "plan-context-v1"@
            || name@ == "collection-plan-v1"@
            || name@ == "full-operations-v1"@
            || name@ == "source-plan-v1"@
            || name@ == "check-unit-assignments-v1"@
            || name@ == "unit-declarations-v1"@
            || name@ == "retained-tool-v1"@
            || name@ == "check-plan-v1"@
            || name@ == "dispatch-plan-v1"@
            || name@ == "check-scoped-plan-v1"@
            || name@ == "scoped-operations-v1"@
            || name@ == "prepare-scoped-plan-v1"@;
        assert((result is Version) == (name@ == "--version"@ && !extra));
        assert((result is Judge) == (!named && (name@ != "--version"@ || extra)));
        assert((result is Usage) == (named && extra));
        assert((result is Query) == (named && !extra));
        if let Command::Query(query) = result {
            match query {
                Query::Evaluate => assert(name@ == "evaluate-routes-v1"@),
                Query::GlobalSelection => assert(name@ == "global-selection"@),
                Query::CollectedSelection => assert(name@ == "collected-selection-v1"@),
                Query::PlanReceipts => assert(name@ == "check-plan-receipts-v1"@),
                Query::OperationReceipt => assert(name@ == "check-operation-receipt-v1"@),
                Query::PlanContext => assert(name@ == "plan-context-v1"@),
                Query::CollectionPlan => assert(name@ == "collection-plan-v1"@),
                Query::FullOperations => assert(name@ == "full-operations-v1"@),
                Query::SourcePlan => assert(name@ == "source-plan-v1"@),
                Query::UnitAssignments => assert(name@ == "check-unit-assignments-v1"@),
                Query::UnitDeclarations => assert(name@ == "unit-declarations-v1"@),
                Query::RetainedTool => assert(name@ == "retained-tool-v1"@),
                Query::CheckPlan => assert(name@ == "check-plan-v1"@),
                Query::DispatchPlan => assert(name@ == "dispatch-plan-v1"@),
                Query::ScopedPlan => assert(name@ == "check-scoped-plan-v1"@),
                Query::ScopedOperations => assert(name@ == "scoped-operations-v1"@),
                Query::PrepareScopedPlan => assert(name@ == "prepare-scoped-plan-v1"@),
            }
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

fn check_merged_operation_order(cyclic: bool)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let a = "A".to_string();
    let b = "B".to_string();
    let mut before_a = BTreeSet::new();
    let mut before_b = BTreeSet::new();
    if cyclic {
        before_a.insert(b.clone());
    }
    before_b.insert(a.clone());
    let mut graph = BTreeMap::new();
    proof {
        reveal_strlit("A");
        reveal_strlit("B");
        assert(a@[0] == 'A' && b@[0] == 'B');
        assert(a != b);
        assert(before_b@ =~= Set::empty().insert(a));
        assert(before_a@ =~= if cyclic { Set::empty().insert(b) } else { Set::empty() });
    }
    graph.insert(a.clone(), (1u8, before_a));
    graph.insert(b.clone(), (2u8, before_b));
    let ghost declared = graph@;
    assert(declared.dom() =~= Set::empty().insert(a).insert(b));
    match policy::order_operations(graph) {
        Ok(ordered) => {
            proof {
                let ids = policy::operation_ids(ordered@);
                assert(ids.to_set().contains(a) && ids.to_set().contains(b));
                assert(ids.len() > 0);
                assert(policy::valid_operation_step(declared, ordered@, 0));
                assert(ids.take(0).to_set() =~= Set::<String>::empty());
                assert(ordered@[0].0 == a);
                assert(0 < ids.len() && exists|i: int| 0 <= i < ids.len() && ids[i] == b);
                let i = choose|i: int| 0 <= i < ids.len() && ids[i] == b;
                assert(i > 0);
                assert(policy::valid_operation_step(declared, ordered@, 1));
                assert(ids.take(1) =~= seq![a]);
                Seq::<String>::empty().lemma_push_to_set_commute(a);
                assert(seq![a] =~= Seq::<String>::empty().push(a));
                assert(ids.take(1).to_set() =~= Set::empty().insert(a));
                assert(ordered@[1].0 == b);
                if ids.len() > 2 {
                    assert(policy::valid_operation_step(declared, ordered@, 2));
                    assert(ids.take(2) =~= seq![a,b]);
                    seq![a].lemma_push_to_set_commute(b);
                    assert(seq![a,b] =~= seq![a].push(b));
                    assert(ids.take(2).to_set() =~= Set::empty().insert(a).insert(b));
                    assert(false);
                }
            }
            expect(!cyclic);
            expect(ordered.len() == 2);
            expect(ordered[0].0 == a && ordered[0].1.0 == 1);
            expect(ordered[1].0 == b && ordered[1].1.0 == 2);
        },
        Err(reason) => {
            proof {
                if !cyclic {
                    reveal(policy::has_blocked_operations);
                    let blocked = choose|blocked: Set<String>| policy::blocked_operations(declared, blocked);
                    assert(!blocked.contains(a));
                    assert(blocked.contains(b));
                    assert(declared.dom().difference(blocked).contains(a));
                    assert(before_b@.subset_of(declared.dom().difference(blocked)));
                    assert(false);
                }
            }
            expect(cyclic);
            expect(reason.to_string() == "E_PLAN_CYCLE: inconsistent operation ordering".to_string());
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn merged_operation_order_rejects_a_cycle_and_orders_the_remaining_edge()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    check_merged_operation_order(true);
    check_merged_operation_order(false);
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
fn registration_views_bind_each_endpoint_in_order() {
    let id = "id".to_string();
    let old = "base".to_string();
    let new = "candidate".to_string();
    proof {
        reveal_strlit("id"); reveal_strlit("base"); reveal_strlit("candidate");
        assert("id"@.len() != "base"@.len());
        assert("base"@.len() != "candidate"@.len());
    }
    expect(policy::view_identity_matches(&id, &id, &old, &new, &old, &new));
    expect(!policy::view_identity_matches(&id, &old, &old, &new, &old, &new));
    expect(!policy::view_identity_matches(&id, &id, &old, &new, &new, &old));
    expect(!policy::view_identity_matches(&id, &id, &old, &new, &old, &old));
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

#[cfg(not(verus_keep_ghost))]
#[test]
fn public_digest_preserves_sha256_and_canonical_key_order() {
    use chrono_judge_routes::{digest, sha256};

    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let value: serde_json::Value = serde_json::from_str(r#"{"z":2,"a":1}"#).unwrap();
    assert_eq!(digest(&value).unwrap(), sha256(br#"{"a":1,"z":2}"#));
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn plan_identity_accepts_the_largest_admitted_timeout() {
    use chrono_judge_routes::{Method, Operation, digest, verified_bounds::valid_execution_bounds};

    let operation = Operation {
        method: Method {
            owner: "example".into(),
            operation: "check".into(),
            tool: "example-tool".into(),
            argv: Vec::new(),
            output_contract: None,
        },
        predecessors: BTreeSet::new(),
        timeout_seconds: u64::MAX,
        output_limit_bytes: 1,
    };
    assert!(valid_execution_bounds(
        operation.timeout_seconds,
        operation.output_limit_bytes
    ));
    digest(&operation).expect("each admitted operation must have a plan identity");
}
