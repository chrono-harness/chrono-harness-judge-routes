//! Decisions used by the route planner and the reader of retained process receipts.
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "units.rs"]
mod units;
#[cfg(not(verus_keep_ghost))]
pub use crate::units::{PlanAssignment, assign_plan, shared_operation_matches, valid_shared_row};
#[cfg(verus_keep_ghost)]
pub use units::{PlanAssignment, assign_plan, shared_operation_matches, valid_shared_row};

#[cfg(verus_keep_ghost)]
#[path = "main.rs"]
pub mod cli;

verus! {

/// Business outcome of a joined runner subprocess and its completed request writer.
pub enum RunnerError {
    ExitStatus,
    Diagnostic(Vec<u8>),
    Request(String),
}

pub fn runner_response(success: bool, stdout: Vec<u8>, stderr: Vec<u8>, sent: Result<(), String>)
    -> (reply: Result<Vec<u8>, RunnerError>)
    ensures match reply {
        Ok(data) => success && sent is Ok && data@ == stdout@,
        Err(RunnerError::ExitStatus) => !success && stderr@.len() == 0,
        Err(RunnerError::Diagnostic(data)) => !success && stderr@.len() > 0 && data@ == stderr@,
        Err(RunnerError::Request(message)) => success && sent == Err::<(), String>(message),
    },
{
    if success {
        match sent {
            Ok(()) => Ok(stdout),
            Err(message) => Err(RunnerError::Request(message)),
        }
    } else if stderr.is_empty() {
        Err(RunnerError::ExitStatus)
    } else {
        Err(RunnerError::Diagnostic(stderr))
    }
}

/// Admit each claimed priority ID at the rank given by its registration order.
pub fn record_priority<K: Ord>(ranks: &mut BTreeMap<K, usize>, id: K, name: &[u8], claimed: bool, rank: usize)
    -> (fresh: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures
        fresh == (claimed && name.len() > 0 && !name@.contains(0u8) && !old(ranks)@.contains_key(id)),
        ranks@ == (if claimed && name.len() > 0 && !name@.contains(0u8) {
            old(ranks)@.insert(id, rank)
        } else { old(ranks)@ }),
{
    if !claimed || name.len() == 0 { return false; }
    let mut i = 0;
    while i < name.len()
        invariant i <= name.len(), forall|j: int| 0 <= j < i ==> name@[j] != 0u8,
        decreases name.len() - i,
    {
        if name[i] == 0 {
            assert(name@.contains(0u8));
            return false;
        }
        i += 1;
    }
    assert(!name@.contains(0u8));
    ranks.insert(id, rank).is_none()
}

/// Stable sorting keeps equal keys (the unranked suffix) in plan order.
pub fn priority_key<K: Ord>(ranks: &BTreeMap<K, usize>, id: &K) -> (key: usize)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures key == (if ranks@.contains_key(*id) { ranks@[*id] } else { usize::MAX }),
{
    match ranks.get(id) { Some(rank) => *rank, None => usize::MAX }
}

pub fn insert_once<K: Ord>(seen: &mut BTreeSet<K>, id: K) -> (fresh: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures seen@ == old(seen)@.insert(id), fresh == !old(seen)@.contains(id),
{
    seen.insert(id)
}

pub fn predecessors_ready<K: Ord>(required: &BTreeSet<K>, finished: &BTreeSet<K>) -> (ready: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures ready == required@.subset_of(finished@),
{
    broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
    let mut items = required.iter();
    loop
        invariant vstd::laws_cmp::obeys_cmp::<K>(), items@.1.to_set() == required@,
            0 <= items@.0 <= items@.1.len(),
            forall|i: int| 0 <= i < items@.0 ==> finished@.contains(#[trigger] items@.1[i]),
        decreases items@.1.len() - items@.0 + 1,
    {
        let item = match items.next() {
            Some(item) => item,
            None => {
                assert forall|key: K| #[trigger] required@.contains(key)
                    implies finished@.contains(key) by {
                    let index = choose|i: int| 0 <= i < items@.1.len() && items@.1[i] == key;
                }
                return true;
            }
        };
        if !finished.contains(item) {
            assert(required@.contains(*item));
            return false;
        }
    }
}

/// A registration decision belongs to the same query only when its nullable
/// request identity and its complete set of retired tests match.
pub fn retirement_reply_matches<K: Ord>(
    expected: Option<&String>, returned: Option<&String>, requested: &BTreeSet<K>, decided: &BTreeSet<K>,
) -> (accepted: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures accepted == (requested@ == decided@ && match (expected, returned) {
        (None, None) => true,
        (Some(a), Some(b)) => a@ == b@,
        _ => false,
    }),
{
    let same_request = match (expected, returned) {
        (None, None) => true,
        (Some(a), Some(b)) => String::eq(a, b),
        _ => false,
    };
    if !same_request || !predecessors_ready(requested, decided) || !predecessors_ready(decided, requested) {
        return false;
    }
    assert(requested@ =~= decided@);
    true
}

pub fn apply_retirement<K: Ord, E>(selected: &mut BTreeSet<K>, result: Result<Option<K>, E>, deferred: bool)
    -> (out: Result<bool, E>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures match result {
        Err(e) => out == Err::<bool, E>(e) && selected@ == old(selected)@,
        Ok(Some(k)) => out == Ok::<bool, E>(true) && selected@ == old(selected)@.insert(k),
        Ok(None) => out == Ok::<bool, E>(deferred) && selected@ == old(selected)@,
    },
{
    match result {
        Err(e) => Err(e),
        Ok(Some(replacement)) => { selected.insert(replacement); Ok(true) },
        Ok(None) => Ok(deferred),
    }
}

pub fn receipt_target_matches(plan: &String, expected_plan: &String, operation: &String, expected_operation: &String)
    -> (matches: bool)
    ensures matches == (plan@ == expected_plan@ && operation@ == expected_operation@),
{
    String::eq(plan, expected_plan) && String::eq(operation, expected_operation)
}

/// Only compute the seal of the whole receipt when the declared method matches.
pub fn receipt_seal_matches<F>(method: &String, calculated: Result<String, String>, receipt: &String, seal: F)
    -> (result: Result<bool, String>)
where
    F: FnOnce() -> Result<String, String>,
    requires seal.requires(()),
    ensures match calculated {
        Err(error) => result == Err::<bool, String>(error),
        Ok(value) => if method@ != value@ { result == Ok::<bool, String>(false) } else {
            exists|d: Result<String, String>| #[trigger] seal.ensures((), d) && match d {
                Err(error) => result == Err::<bool, String>(error),
                Ok(digest) => result == Ok::<bool, String>(receipt@ == digest@),
            }
        },
    },
{
    let value = calculated?;
    if !String::eq(method, &value) { return Ok(false); }
    let digest = seal()?;
    Ok(String::eq(receipt, &digest))
}

/// The wire adapter uses Rust path equality for the working directory and executable.
pub fn receipt_tool_matches(same_paths: bool, observed: &String, expected: &String) -> (matches: bool)
    ensures matches == (same_paths && observed@ == expected@),
{
    same_paths && String::eq(observed, expected)
}

/// Map equality precedes its canonical digest; stdin is inspected last.
pub fn receipt_input_matches<H, I>(same_environment: bool, recorded: &String, hash: H, stdin: &String, empty: I)
    -> (result: Result<bool, String>)
where
    H: FnOnce() -> Result<String, String>, I: FnOnce() -> String,
    requires hash.requires(()), empty.requires(()),
    ensures if !same_environment { result == Ok::<bool, String>(false) } else {
        exists|d: Result<String, String>| #[trigger] hash.ensures((), d) && match d {
            Err(error) => result == Err::<bool, String>(error),
            Ok(value) => if recorded@ != value@ { result == Ok::<bool, String>(false) } else {
                exists|s: String| #[trigger] empty.ensures((), s) && result == Ok::<bool, String>(stdin@ == s@)
            }
        }
    },
{
    if !same_environment { return Ok(false); }
    let digest = hash()?;
    Ok(String::eq(recorded, &digest) && String::eq(stdin, &empty()))
}

pub fn compiler_check_required(contract: bool, dependency: bool, artifact: bool) -> (required: bool)
    ensures required == (contract || dependency || artifact),
{
    contract || dependency || artifact
}

pub fn output_sizes_fit(stdout: usize, stderr: usize, limit: usize) -> (fit: bool)
    ensures fit == (stdout <= limit && stderr <= limit),
{
    stdout <= limit && stderr <= limit
}

pub fn stream_matches(digest: &String, calculated: &String, decoded_matches: bool) -> (matches: bool)
    ensures matches == (digest@ == calculated@ && decoded_matches),
{
    String::eq(digest, calculated) && decoded_matches
}

/// Check a completed version probe before accepting its recorded environment.
pub fn probe_matches<O, S, E>(
    exit_code: i32, failed: bool, stdout: O, stderr: S, environment: E,
) -> (result: Result<bool, String>)
where
    O: FnOnce() -> bool,
    S: FnOnce() -> bool,
    E: FnOnce() -> Result<bool, String>,
    requires stdout.requires(()), stderr.requires(()), environment.requires(()),
    ensures
        if exit_code != 0 || failed {
            result == Ok::<bool, String>(false)
        } else {
            exists|out: bool| #[trigger] stdout.ensures((), out) &&
                if !out {
                    result == Ok::<bool, String>(false)
                } else {
                    exists|err: bool| #[trigger] stderr.ensures((), err) &&
                        if !err { result == Ok::<bool, String>(false) }
                        else { environment.ensures((), result) }
                }
        },
{
    if exit_code != 0 || failed || !stdout() || !stderr() { return Ok(false); }
    environment()
}

}

/// Pure validation of successful retained observations, shared with full consumers.
#[cfg(not(verus_keep_ghost))]
pub fn process_success(p: &crate::ProcessResult) -> Result<(), String> {
    let matches = probe_matches(
        p.exit_code,
        p.failure.is_some(),
        || {
            stream_matches(
                &p.stdout_sha256,
                &crate::sha256(&p.stdout_bytes),
                p.stdout == String::from_utf8_lossy(&p.stdout_bytes),
            )
        },
        || {
            stream_matches(
                &p.stderr_sha256,
                &crate::sha256(&p.stderr_bytes),
                p.stderr == String::from_utf8_lossy(&p.stderr_bytes),
            )
        },
        || Ok(p.environment_digest == crate::digest(&p.environment)?),
    )?;
    if matches {
        Ok(())
    } else {
        Err("E_PROCESS_EVIDENCE: unsuccessful or inconsistent retained process".into())
    }
}
