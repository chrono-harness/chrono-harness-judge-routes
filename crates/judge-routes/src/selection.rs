#[cfg(verus_keep_ghost)]
use super::{predecessors_ready, protocol::request_matches};
#[cfg(not(verus_keep_ghost))]
use crate::{protocol::request_matches, verified_admission::predecessors_ready};
use std::collections::BTreeSet;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
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
    if !request_matches(expected, returned) || !predecessors_ready(requested, decided) || !predecessors_ready(decided, requested) {
        return false;
    }
    assert(requested@ =~= decided@);
    true
}

}

#[cfg(not(verus_keep_ghost))]
use crate::protocol::Request;
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

/// The planner chooses the global obligations before applying a transport scope.
#[cfg(not(verus_keep_ghost))]
fn merge(
    tests: Vec<String>,
    retired: &[String],
    deferred: bool,
    mut resolve: impl FnMut(&str) -> Result<Option<String>, String>,
) -> Result<BTreeSet<String>, String> {
    let mut selected: BTreeSet<_> = tests.into_iter().collect();
    for test in retired {
        if !crate::verified_admission::apply_retirement(&mut selected, resolve(test), deferred)? {
            return Err(format!("E_REQUIRED_TEST_REMOVED: {test}"));
        }
    }
    Ok(selected)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Documents {
    config: String,
    base: BTreeMap<String, Value>,
    candidate: BTreeMap<String, Value>,
    tests: Vec<String>,
    retired_tests: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    request: Request,
    migration: Option<Value>,
    tests: Vec<String>,
    retired_tests: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Decisions {
    request_id: Value,
    downstream: bool,
    retirements: BTreeMap<String, Result<Option<String>, String>>,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn choose(
    judge: &str,
    endpoints: Value,
    expected: Option<&String>,
    tests: Vec<String>,
    retired: Vec<String>,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<BTreeSet<String>, String> {
    let query = json!({"judge":judge,"endpoints":endpoints,"retired_tests":retired});
    let reply: Decisions =
        serde_json::from_slice(&run("test-retirements-v1", &query)?).map_err(|e| e.to_string())?;
    let mismatch = "E_RETIREMENT: registration reply differs from requested tests or collection";
    let returned = match &reply.request_id {
        Value::Null => None,
        Value::String(id) => Some(id),
        _ => return Err(mismatch.into()),
    };
    if !retirement_reply_matches(
        expected,
        returned,
        &retired.iter().collect(),
        &reply.retirements.keys().collect(),
    ) {
        return Err(mismatch.into());
    }
    merge(tests, &retired, reply.downstream, |test| {
        reply.retirements[test].clone()
    })
}

#[cfg(not(verus_keep_ghost))]
fn write_selection(
    endpoints: Value,
    expected: Option<&String>,
    tests: Vec<String>,
    retired: Vec<String>,
    mut output: impl Write,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let selected = choose("routes", endpoints, expected, tests, retired, run)?;
    serde_json::to_writer(&mut output, &selected).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn select_from_registries_with(
    input: impl Read,
    output: impl Write,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let q: Documents = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    write_selection(
        json!({"mode":"documents","config":q.config,"base":q.base,"candidate":q.candidate}),
        None,
        q.tests,
        q.retired_tests,
        output,
        run,
    )
}

#[cfg(not(verus_keep_ghost))]
pub fn select_collected_with(
    root: &Path,
    input: impl Read,
    output: impl Write,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let q: Collection = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    if q.request.candidate.root != root
        || !matches!(
            q.request.scope,
            Some(crate::protocol::Scope::Collect { .. })
        )
    {
        return Err("route collection requires this checkout and a collection scope".into());
    }
    write_selection(
        json!({"mode":"collection","request":q.request,"migration":q.migration}),
        Some(&q.request.request_id),
        q.tests,
        q.retired_tests,
        output,
        run,
    )
}

#[cfg(not(verus_keep_ghost))]
pub fn select_from_registries(input: impl Read, output: impl Write) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    select_from_registries_with(input, output, |operation, query| {
        crate::registration::query(&root, operation, query)
    })
}

#[cfg(not(verus_keep_ghost))]
pub fn select_collected(root: &Path, input: impl Read, output: impl Write) -> Result<(), String> {
    select_collected_with(root, input, output, |operation, query| {
        crate::registration::query(root, operation, query)
    })
}
