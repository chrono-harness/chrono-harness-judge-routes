//! Global assignment and scoped selection for route plans.
#[cfg(verus_keep_ghost)]
use super::{insert_once, predecessors_ready};
#[cfg(not(verus_keep_ghost))]
use crate::verified_admission::{insert_once, predecessors_ready};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum PlanAssignment { Accepted, Unknown, Duplicate }

pub fn assign_plan<K: Ord>(plans: &BTreeSet<K>, assigned: &mut BTreeSet<K>, plan: K)
    -> (result: PlanAssignment)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures match result {
        PlanAssignment::Unknown => !plans@.contains(plan) && assigned@ == old(assigned)@,
        PlanAssignment::Duplicate => plans@.contains(plan) && old(assigned)@.contains(plan)
            && assigned@ == old(assigned)@,
        PlanAssignment::Accepted => plans@.contains(plan) && !old(assigned)@.contains(plan)
            && assigned@ == old(assigned)@.insert(plan),
    },
{
    broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
    if !plans.contains(&plan) {
        PlanAssignment::Unknown
    } else if insert_once(assigned, plan) {
        PlanAssignment::Accepted
    } else {
        assert(assigned@ =~= old(assigned)@);
        PlanAssignment::Duplicate
    }
}

/// A shared row must name at least two distinct registered units, without repeated names.
pub fn valid_shared_row<K: Ord, V>(members: &BTreeSet<K>, list_len: usize, units: &BTreeMap<K,V>)
    -> (valid: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures valid == (members@.len() == list_len && list_len >= 2 && members@.subset_of(units@.dom())),
{
    broadcast use vstd::std_specs::btree::axiom_spec_btree_set_len;
    broadcast use vstd::std_specs::btree::axiom_contains_deref_key;
    if members.len() != list_len || list_len < 2 { return false; }
    let mut items = members.iter();
    loop
        invariant vstd::laws_cmp::obeys_cmp::<K>(), members@.len() == list_len, list_len >= 2,
            items@.1.to_set() == members@,
            0 <= items@.0 <= items@.1.len(),
            forall|i: int| 0 <= i < items@.0 ==> units@.contains_key(#[trigger] items@.1[i]),
        decreases items@.1.len() - items@.0 + 1,
    {
        let key = match items.next() {
            Some(key) => key,
            None => {
                assert forall|k: K| #[trigger] members@.contains(k) implies units@.contains_key(k) by {
                    let i = choose|i: int| 0 <= i < items@.1.len() && items@.1[i] == k;
                }
                return true;
            }
        };
        if !units.contains_key(key) {
            assert(members@.contains(*key));
            return false;
        }
    }
}

/// An operation repeated by several units needs one declaration with exactly those members.
pub fn shared_operation_matches<K: Ord>(actual: &BTreeSet<K>, declared: Option<&BTreeSet<K>>)
    -> (matched: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures matched == (match declared {
        None => actual@.len() <= 1,
        Some(members) => actual@.len() > 1 && actual@ == members@,
    }),
{
    broadcast use vstd::std_specs::btree::axiom_spec_btree_set_len;
    match declared {
        None => actual.len() <= 1,
        Some(members) => {
            if actual.len() <= 1 || !predecessors_ready(actual, members) || !predecessors_ready(members, actual) {
                return false;
            }
            assert(actual@ =~= members@);
            true
        }
    }
}

}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub tests: Vec<String>,
    pub report_path: String,
}

/// Validate and select the explicitly registered full execution units.  This
/// is deliberately a pure map operation shared by the full route and the
/// scoped adapter; it never discovers plans or derives ownership from paths.
#[cfg(not(verus_keep_ghost))]
pub fn assignments(
    units: &BTreeMap<String, Unit>,
    plans: &BTreeSet<String>,
    shared: &BTreeMap<String, Vec<String>>,
    operation_sets: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut assigned = BTreeSet::new();
    for (unit, definition) in units {
        id(unit)?;
        for test in &definition.tests {
            match assign_plan(plans, &mut assigned, test.clone()) {
                PlanAssignment::Accepted => {}
                PlanAssignment::Unknown => return Err(format!("unknown CI unit plan: {test}")),
                PlanAssignment::Duplicate => {
                    return Err(format!("duplicate CI unit plan assignment: {test}"));
                }
            }
        }
    }
    if !predecessors_ready(plans, &assigned) {
        return Err("missing CI unit plan assignment".into());
    }
    let mut expected = BTreeMap::<String, BTreeSet<String>>::new();
    for (operation, names) in shared {
        let set: BTreeSet<_> = names.iter().cloned().collect();
        if !valid_shared_row(&set, names.len(), units) {
            return Err(format!("invalid shared operation assignment: {operation}"));
        }
        expected.insert(operation.clone(), set);
    }
    if operation_sets
        .iter()
        .any(|(op, members)| !shared_operation_matches(members, expected.get(op)))
        || expected.keys().any(|op| !operation_sets.contains_key(op))
    {
        return Err(
            "shared operations must explicitly name exactly the units that repeat them".into(),
        );
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub fn select(
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
    unit: &str,
) -> Result<BTreeSet<String>, String> {
    id(unit)?;
    Ok(units
        .get(unit)
        .ok_or("unregistered CI unit")?
        .tests
        .iter()
        .filter(|test| selected.contains(*test))
        .cloned()
        .collect())
}

#[cfg(not(verus_keep_ghost))]
pub fn required(units: &BTreeMap<String, Unit>, selected: &BTreeSet<String>) -> Vec<String> {
    units
        .iter()
        .filter(|(_, definition)| definition.tests.iter().any(|test| selected.contains(test)))
        .map(|(id, _)| id.clone())
        .collect()
}

#[cfg(not(verus_keep_ghost))]
fn id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(format!("invalid CI unit ID: {id:?}"));
    }
    Ok(())
}
