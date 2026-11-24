//! Decisions used by the route planner and the reader of retained process receipts.
use std::collections::BTreeSet;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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

}
