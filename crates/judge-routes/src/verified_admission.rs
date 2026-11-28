//! Decisions used by the route planner and the reader of retained process receipts.
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "lib.rs"]
pub mod library;

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

#[cfg(verus_keep_ghost)]
#[path = "plan_query.rs"]
pub mod plan_query;

#[cfg(verus_keep_ghost)]
#[path = "source_plan.rs"]
pub mod source_plan;

#[cfg(verus_keep_ghost)]
#[path = "protocol.rs"]
pub mod protocol;
#[cfg(verus_keep_ghost)]
#[path = "registration.rs"]
pub mod registration;
#[cfg(verus_keep_ghost)]
#[path = "selection.rs"]
pub mod selection;
#[cfg(not(verus_keep_ghost))]
pub use crate::registration::view_identity_matches;
#[cfg(not(verus_keep_ghost))]
pub use crate::selection::retirement_reply_matches;
#[cfg(verus_keep_ghost)]
pub use registration::view_identity_matches;
#[cfg(verus_keep_ghost)]
pub use selection::retirement_reply_matches;

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

/// A node carries its original operation data and the union of its declared predecessors.
pub type OperationGraph<V> = BTreeMap<String, (V, BTreeSet<String>)>;

/// Resolve a declaration without substituting for a missing route.
pub fn required_declaration<'a, T, E: Copy>(declaration: Option<&'a T>, error: E)
    -> (result: Result<&'a T, E>)
    ensures result == match declaration { Some(value) => Ok(value), None => Err(error) },
{
    match declaration { Some(value) => Ok(value), None => Err(error) }
}

/// Only one execute binding may enter the selected-action map.
pub fn unique_action<T>(actions: &[T]) -> (action: Option<&T>)
    ensures action == if actions@.len() == 1 { Some(&actions[0]) } else { None },
{
    if actions.len() == 1 { Some(&actions[0]) } else { None }
}

/// Match the original execute binding to a member of the declared sequence.
pub fn contains_action(operations: &[String], action: &String) -> (present: bool)
    ensures present == (exists|i: int| 0 <= i < operations@.len()
        && (#[trigger] operations@[i])@ == action@),
{
    let mut i = 0;
    while i < operations.len()
        invariant
            i <= operations@.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] operations@[j])@ != action@,
        decreases operations@.len() - i,
    {
        if <String as PartialEq>::eq(&operations[i], action) {
            assert(operations@[i as int]@ == action@);
            return true;
        }
        i += 1;
    }
    false
}

pub enum MethodIssue {
    Missing,
    Ambiguous,
}

/// Missing methods precede the reserved name and cardinality checks.
pub fn admit_method<'a, T>(id: &str, definitions: Option<&'a Vec<T>>)
    -> (method: Result<&'a T, MethodIssue>)
    ensures match definitions {
        None => method is Err && method->Err_0 is Missing,
        Some(defs) => if defs@.len() == 1 && id@ != "validate.delta"@ {
            method matches Ok(value) && *value == defs@[0]
        } else {
            method is Err && method->Err_0 is Ambiguous
        },
    },
{
    match definitions {
        None => Err(MethodIssue::Missing),
        Some(defs) => {
            if defs.len() != 1 || <&str>::exec_eq(id, "validate.delta") {
                Err(MethodIssue::Ambiguous)
            } else {
                Ok(&defs[0])
            }
        },
    }
}

/// A later declaration contributes one predecessor and cannot replace the retained method.
pub fn merge_operation<T>(
    graph: &mut OperationGraph<(T, u64, usize)>,
    id: String,
    declaration: (T, u64, usize),
    previous: Option<String>,
) -> (admitted: bool)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures
        admitted == (!old(graph)@.contains_key(id)
            || (old(graph)@[id].0.1 == declaration.1 && old(graph)@[id].0.2 == declaration.2)),
        !admitted ==> graph@ == old(graph)@,
        admitted ==> graph@.dom() =~= old(graph)@.dom().insert(id),
        admitted ==> forall|other: String| other != id && #[trigger] old(graph)@.contains_key(other)
            ==> graph@[other] == old(graph)@[other],
        admitted ==> graph@[id].0 == if old(graph)@.contains_key(id) { old(graph)@[id].0 } else { declaration },
        admitted ==> graph@[id].1@ =~= {
            let before = if old(graph)@.contains_key(id) { old(graph)@[id].1@ } else { Set::empty() };
            match previous { Some(p) => before.insert(p), None => before }
        },
{
    broadcast use vstd::std_specs::btree::axiom_deref_key_removed;
    broadcast use vstd::std_specs::btree::axiom_maps_deref_key_to_value;
    let ghost before = graph@;
    let (method, mut predecessors) = match graph.remove(&id) {
        Some(node) => {
            if node.0.1 != declaration.1 || node.0.2 != declaration.2 {
                graph.insert(id, node);
                assert(graph@ =~= before);
                return false;
            }
            node
        },
        None => (declaration, BTreeSet::new()),
    };
    if let Some(predecessor) = previous {
        predecessors.insert(predecessor);
    }
    graph.insert(id, (method, predecessors));
    assert(graph@.dom() =~= before.dom().insert(id));
    true
}

fn first_ready<'a, V>(pending: &'a OperationGraph<V>, finished: &BTreeSet<String>)
    -> (ready: Option<&'a String>)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures match ready {
        Some(id) => {
            &&& pending@.contains_key(*id)
            &&& pending@[*id].1@.subset_of(finished@)
            &&& forall|other: String| #[trigger] pending@.contains_key(other)
                && pending@[other].1@.subset_of(finished@) && other != *id ==>
                    vstd::std_specs::cmp::OrdSpec::cmp_spec(id, &other) is Less
        },
        None => forall|id: String| #[trigger] pending@.contains_key(id) ==>
            !pending@[id].1@.subset_of(finished@),
    },
{
    broadcast use vstd::std_specs::btree::axiom_key_obeys_cmp_spec_meaning;
    broadcast use vstd::std_specs::btree::axiom_increasing_seq_meaning;
    let mut entries = pending.iter();
    let ghost rows = entries@.1;
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            entries@.1 == rows,
            rows.to_set() == pending@.kv_pairs(),
            vstd::std_specs::btree::increasing_seq(rows.map(|i: int, row: (String, (V, BTreeSet<String>))| row.0)),
            0 <= entries@.0 <= rows.len(),
            forall|i: int| 0 <= i < entries@.0 ==> !(#[trigger] rows[i]).1.1@.subset_of(finished@),
        decreases rows.len() - entries@.0 + 1,
    {
        let ghost index = entries@.0;
        match entries.next() {
            Some((id, (_, required))) => {
                if predecessors_ready(required, finished) {
                    assert forall|other: String| #[trigger] pending@.contains_key(other)
                        && pending@[other].1@.subset_of(finished@) && other != *id implies
                            vstd::std_specs::cmp::OrdSpec::cmp_spec(id, &other) is Less by {
                        let item = (other, pending@[other]);
                        assert(rows.to_set().contains(item));
                        let j = choose|j: int| 0 <= j < rows.len() && rows[j] == item;
                        assert(j > index);
                        let keys = rows.map(|i: int, row: (String, (V, BTreeSet<String>))| row.0);
                        assert(keys[index] == *id && keys[j] == other);
                    }
                    return Some(id);
                }
            },
            None => {
                assert forall|id: String| #[trigger] pending@.contains_key(id) implies
                    !pending@[id].1@.subset_of(finished@) by {
                    let item = (id, pending@[id]);
                    assert(rows.to_set().contains(item));
                    let j = choose|j: int| 0 <= j < rows.len() && rows[j] == item;
                }
                return None;
            },
        }
    }
}

/// Schedule the smallest ready key each time. A blocked remainder cannot be scheduled.
pub fn order_operations<V>(graph: OperationGraph<V>)
    -> (result: Result<Vec<(String, (V, BTreeSet<String>))>, &'static str>)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures
        result matches Ok(ordered) ==> operation_ids(ordered@).to_set() =~= graph@.dom(),
        result matches Ok(ordered) ==> valid_operation_prefix(graph@, ordered@),
        result matches Err(reason) ==> reason@ == "E_PLAN_CYCLE: inconsistent operation ordering"@,
        result is Err ==> has_blocked_operations(graph@),
{
    broadcast use vstd::std_specs::btree::axiom_key_obeys_cmp_spec_meaning;
    broadcast use vstd::std_specs::btree::axiom_deref_key_removed;
    broadcast use vstd::std_specs::btree::axiom_maps_deref_key_to_value;
    let ghost graph_view = graph@;
    let mut pending = graph;
    let mut ordered = Vec::new();
    let mut finished = BTreeSet::new();
    while pending.len() != 0
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            graph_view == graph@,
            finished@ =~= operation_ids(ordered@).to_set(),
            finished@.subset_of(graph_view.dom()),
            valid_operation_prefix(graph_view, ordered@),
            pending@.dom() =~= graph_view.dom().difference(finished@),
            forall|id: String| #[trigger] pending@.contains_key(id) ==> pending@[id] == graph_view[id],
        decreases pending@.len(),
    {
        let id = match first_ready(&pending, &finished) {
            Some(id) => id.clone(),
            None => {
                let ghost blocked = pending@.dom();
                assert(blocked.subset_of(graph_view.dom()));
                assert(!blocked.is_empty());
                assert(graph_view.dom().difference(blocked) =~= finished@);
                assert forall|id: String| #[trigger] blocked.contains(id) implies
                    !graph_view[id].1@.subset_of(graph_view.dom().difference(blocked)) by {
                    assert(pending@.contains_key(id));
                }
                assert(blocked_operations(graph_view, blocked));
                reveal(has_blocked_operations);

                assert(has_blocked_operations(graph_view));
                assert(has_blocked_operations(graph@));
                let reason = "E_PLAN_CYCLE: inconsistent operation ordering";
                return Err(reason);
            },
        };
        let ghost before = pending@;
        let node = pending.remove(&id).unwrap();
        let ghost prefix = ordered@;
        let ghost placed = finished@;
        finished.insert(id.clone());
        ordered.push((id, node));
        assert(operation_ids(ordered@) =~= operation_ids(prefix).push(id));
        proof { operation_ids(prefix).lemma_push_to_set_commute(id); }
        assert(finished@ =~= operation_ids(ordered@).to_set());
        assert(pending@.dom() =~= before.dom().remove(id));
        assert(pending@.dom() =~= graph_view.dom().difference(finished@));
        assert(valid_operation_prefix(graph_view, ordered@)) by {
            assert forall|i: int| 0 <= i < ordered@.len() implies
                valid_operation_step(graph_view, ordered@, i) by {
                if i < prefix.len() {
                    assert(ordered@[i] == prefix[i]);
                    assert(operation_ids(ordered@).take(i) =~= operation_ids(prefix).take(i));
                    assert(valid_operation_step(graph_view, prefix, i));
                } else {
                    assert(operation_ids(ordered@).take(i) =~= operation_ids(prefix));
                    assert(operation_ids(prefix).to_set() =~= placed);
                    assert(ordered@[i] == (id, node));
                    assert(before.contains_key(id));
                    assert forall|other: String| #[trigger] graph_view.contains_key(other)
                        && !placed.contains(other) && graph_view[other].1@.subset_of(placed)
                        && other != id implies
                            vstd::std_specs::cmp::OrdSpec::cmp_spec(&id, &other) is Less by {
                        assert(before.contains_key(other));
                    }
                }
            }
        }
    }
    assert(pending@.dom().len() == 0);
    assert(pending@.dom() =~= Set::<String>::empty());
    assert forall|id: String| #[trigger] graph_view.dom().contains(id) implies finished@.contains(id) by {
        if !finished@.contains(id) {
            assert(graph_view.dom().difference(finished@).contains(id));
            assert(pending@.dom().contains(id));
        }
    }
    assert(operation_ids(ordered@).to_set() =~= graph_view.dom());
    Ok(ordered)
}

#[verifier::opaque]
pub open spec fn has_blocked_operations<V>(graph: Map<String, (V, BTreeSet<String>)>) -> bool {
    exists|blocked: Set<String>| #[trigger] blocked_operations(graph, blocked)
}

pub open spec fn blocked_operations<V>(graph: Map<String, (V, BTreeSet<String>)>, blocked: Set<String>) -> bool {
    &&& blocked.subset_of(graph.dom())
    &&& !blocked.is_empty()
    &&& forall|id: String| #[trigger] blocked.contains(id) ==>
        !graph[id].1@.subset_of(graph.dom().difference(blocked))
}

pub open spec fn operation_ids<V>(rows: Seq<(String, (V, BTreeSet<String>))>) -> Seq<String> {
    rows.map(|i: int, row: (String, (V, BTreeSet<String>))| row.0)
}

pub open spec fn valid_operation_step<V>(
    graph: Map<String, (V, BTreeSet<String>)>, rows: Seq<(String, (V, BTreeSet<String>))>, i: int,
) -> bool {
    let (id, node) = rows[i];
    let before = operation_ids(rows).take(i).to_set();
    &&& graph.contains_key(id) && graph[id] == node
    &&& !before.contains(id) && node.1@.subset_of(before)
    &&& forall|other: String| #[trigger] graph.contains_key(other)
        && !before.contains(other) && graph[other].1@.subset_of(before) && other != id ==>
            vstd::std_specs::cmp::OrdSpec::cmp_spec(&id, &other) is Less
}

pub open spec fn valid_operation_prefix<V>(
    graph: Map<String, (V, BTreeSet<String>)>, rows: Seq<(String, (V, BTreeSet<String>))>,
) -> bool {
    forall|i: int| 0 <= i < rows.len() ==> #[trigger] valid_operation_step(graph, rows, i)
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
    plan_identity_matches(plan, expected_plan) && String::eq(operation, expected_operation)
}

pub fn plan_identity_matches(plan: &String, expected: &String) -> (matches: bool)
    ensures matches == (plan@ == expected@),
{
    String::eq(plan, expected)
}

/// Standard-library path, argv and map comparisons describe the original invocation.
pub struct VersionInvocation {
    pub absolute_path: bool,
    pub same_basename: bool,
    pub same_executable: bool,
    pub same_argv: bool,
    pub same_directory: bool,
    pub same_environment: bool,
    pub same_stdout_bytes: bool,
}

pub fn retained_version_matches(
    invocation: VersionInvocation,
    program: &str,
    expected_program: &str,
    digest: &String,
    observed_digest: &String,
    version: &str,
    expected_version: Option<&str>,
) -> (matches: bool)
    ensures matches == (
        program@ == expected_program@ && invocation.absolute_path && invocation.same_basename
        && invocation.same_executable && digest@ == observed_digest@ && invocation.same_argv
        && invocation.same_directory && invocation.same_environment
        && (expected_version matches Some(expected) && version@ == expected@)
        && invocation.same_stdout_bytes
    ),
{
    if !<&str>::exec_eq(program, expected_program)
        || !invocation.absolute_path || !invocation.same_basename
        || !receipt_tool_matches(invocation.same_executable, digest, observed_digest)
        || !invocation.same_argv || !invocation.same_directory || !invocation.same_environment
    {
        return false;
    }
    match expected_version {
        Some(expected) => <&str>::exec_eq(version, expected) && invocation.same_stdout_bytes,
        None => false,
    }
}

/// An offline observation is sufficient only for collection. Live planning also verifies the file.
pub fn accept_tool_version<F>(matches: bool, live: bool, file_matches: F) -> (result: Result<(), &'static str>)
where
    F: FnOnce() -> bool,
    requires file_matches.requires(()),
    ensures
        !matches ==> result is Err,
        matches && !live ==> result is Ok,
        matches && live ==> exists|same: bool| #[trigger] file_matches.ensures((), same)
            && (result is Ok) == same,
        (result matches Err(e) ==> e@ == "E_TOOL_BINDING: path/version/digest/environment observation mismatch"@),
{
    if matches && (!live || file_matches()) {
        Ok(())
    } else {
        Err("E_TOOL_BINDING: path/version/digest/environment observation mismatch")
    }
}

pub fn retain_or_observe<T, E, F>(retained: Option<T>, observe: F) -> (result: Result<T, E>)
where
    F: FnOnce() -> Result<T, E>,
    requires observe.requires(()),
    ensures match retained {
        Some(tool) => result == Ok::<T, E>(tool),
        None => observe.ensures((), result),
    },
{
    match retained {
        Some(tool) => Ok(tool),
        None => observe(),
    }
}

/// A file from either registered source can supply an operation's tool.
pub fn admit_tool_origin<F, G>(external_input: F, candidate_file: G) -> (registered: bool)
where
    F: FnOnce() -> bool,
    G: FnOnce() -> bool,
    requires external_input.requires(()), candidate_file.requires(()),
    ensures exists|external: bool| #[trigger] external_input.ensures((), external) &&
        if external {
            registered
        } else {
            exists|candidate: bool| #[trigger] candidate_file.ensures((), candidate)
                && registered == candidate
        },
{
    external_input() || candidate_file()
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
