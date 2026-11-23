use chrono_harness::wire::Request;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
};

/// The planner chooses the global obligations before applying a transport scope.
fn merge(
    tests: Vec<String>,
    retired: &[String],
    deferred: bool,
    mut resolve: impl FnMut(&str) -> Result<Option<String>, String>,
) -> Result<BTreeSet<String>, String> {
    let mut selected: BTreeSet<_> = tests.into_iter().collect();
    for test in retired {
        match resolve(test)? {
            Some(replacement) => {
                selected.insert(replacement);
            }
            None if deferred => {}
            None => return Err(format!("E_REQUIRED_TEST_REMOVED: {test}")),
        }
    }
    Ok(selected)
}

#[derive(Deserialize)]
struct Documents {
    config: String,
    base: BTreeMap<String, Value>,
    candidate: BTreeMap<String, Value>,
    tests: Vec<String>,
    retired_tests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    request: Request,
    migration: Option<Value>,
    tests: Vec<String>,
    retired_tests: Vec<String>,
}

#[derive(Deserialize)]
struct Decisions {
    request_id: Value,
    downstream: bool,
    retirements: BTreeMap<String, Result<Option<String>, String>>,
}

pub(crate) fn choose(
    judge: &str,
    endpoints: Value,
    expected: Value,
    tests: Vec<String>,
    retired: Vec<String>,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<BTreeSet<String>, String> {
    let query = json!({"judge":judge,"endpoints":endpoints,"retired_tests":retired});
    let reply: Decisions =
        serde_json::from_slice(&run("test-retirements-v1", &query)?).map_err(|e| e.to_string())?;
    if reply.request_id != expected
        || reply.retirements.keys().collect::<BTreeSet<_>>() != retired.iter().collect()
    {
        return Err(
            "E_RETIREMENT: registration reply differs from requested tests or collection".into(),
        );
    }
    merge(tests, &retired, reply.downstream, |test| {
        reply.retirements[test].clone()
    })
}

fn write_selection(
    endpoints: Value,
    expected: Value,
    tests: Vec<String>,
    retired: Vec<String>,
    mut output: impl Write,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let selected = choose("routes", endpoints, expected, tests, retired, run)?;
    serde_json::to_writer(&mut output, &selected).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

pub fn select_from_registries_with(
    input: impl Read,
    output: impl Write,
    run: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let q: Documents = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    write_selection(
        json!({"mode":"documents","config":q.config,"base":q.base,"candidate":q.candidate}),
        Value::Null,
        q.tests,
        q.retired_tests,
        output,
        run,
    )
}

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
            Some(chrono_harness::units::Scope::Collect { .. })
        )
    {
        return Err("route collection requires this checkout and a collection scope".into());
    }
    let expected = json!(q.request.request_id);
    write_selection(
        json!({"mode":"collection","request":q.request,"migration":q.migration}),
        expected,
        q.tests,
        q.retired_tests,
        output,
        run,
    )
}

pub fn select_from_registries(input: impl Read, output: impl Write) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    select_from_registries_with(input, output, |operation, query| {
        crate::registration::query(&root, operation, query)
    })
}

pub fn select_collected(root: &Path, input: impl Read, output: impl Write) -> Result<(), String> {
    select_collected_with(root, input, output, |operation, query| {
        crate::registration::query(root, operation, query)
    })
}
