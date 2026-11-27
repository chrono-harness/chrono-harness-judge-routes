//! Run a public query on the host-selected registration executable.
#[cfg(verus_keep_ghost)]
use super::protocol::request_matches;
#[cfg(not(verus_keep_ghost))]
use crate::protocol::request_matches;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Route views bind both commits in the order used to resolve changes.
pub fn view_identity_matches(
    expected_id: &String, returned_id: &String, base: &String, candidate: &String,
    returned_base: &String, returned_candidate: &String,
) -> (matched: bool)
    ensures matched == (expected_id@ == returned_id@ && base@ == returned_base@
        && candidate@ == returned_candidate@),
{
    request_matches(Some(expected_id), Some(returned_id)) &&
        String::eq(base, returned_base) && String::eq(candidate, returned_candidate)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{Method, Plan, Scheduling};
#[cfg(not(verus_keep_ghost))]
use crate::{Tool, protocol::Request};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
pub(crate) fn query(root: &Path, operation: &str, query: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(
        "installed-query-v1",
        &json!({"root":root,"program":"chrono-judge-registration",
        "operation":operation,"environment":"search-path","input":query}),
    )
}

#[cfg(not(verus_keep_ghost))]
fn native_query(req: &Request, operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(
        "installed-query-v1",
        &json!({"root":req.candidate.root,"program":"chrono-judge-registration",
        "operation":operation,"environment":"inherited","input":input}),
    )
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn runner(_root: &Path, operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(operation, input)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Endpoint {
    pub config: Value,
    pub filemap: Value,
    pub projects: Value,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Views {
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub methods: Result<BTreeMap<String, Vec<Method>>, String>,
    pub plans: Result<BTreeMap<String, Plan>, String>,
    pub test_operations: BTreeMap<String, Vec<String>>,
    pub scheduling: Result<Option<Scheduling>, String>,
    pub reused: Result<BTreeMap<String, Tool>, String>,
    pub registration_view: Value,
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn views(req: &Request, git: &mut Value) -> Result<Views, String> {
    #[derive(Deserialize)]
    struct Reply {
        request_id: String,
        commits: [String; 2],
        git_facts: Option<Value>,
        result: Result<Views, String>,
    }
    let bytes = native_query(req, "route-views-v1", &json!(req))?;
    let reply: Reply = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !view_identity_matches(
        &req.request_id,
        &reply.request_id,
        &req.base.commit,
        &req.candidate.commit,
        &reply.commits[0],
        &reply.commits[1],
    ) {
        return Err("E_PROTOCOL: registration route views identity".into());
    }
    *git = reply.git_facts.unwrap_or(Value::Null);
    reply.result
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn effective_inputs(
    req: &Request,
    view: &Value,
    selected: &BTreeSet<String>,
) -> Result<Value, String> {
    #[derive(Deserialize)]
    struct Reply {
        request_id: String,
        effective_inputs: Value,
    }
    let bytes = native_query(
        req,
        "route-inputs-v1",
        &json!({"request":req,"view":view,"selected":selected}),
    )?;
    let reply: Reply = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !request_matches(Some(&req.request_id), Some(&reply.request_id)) {
        return Err("E_PROTOCOL: registration route inputs identity".into());
    }
    Ok(reply.effective_inputs)
}

/// Registration reads the fixed endpoints first. The route planner may read source continuity later.
#[cfg(not(verus_keep_ghost))]
pub(super) fn append_source_facts(
    registration: &mut Value,
    mut source: Value,
) -> Result<(), String> {
    if registration.is_null() && source.is_null() {
        return Ok(());
    }
    let first = registration
        .as_object_mut()
        .ok_or("E_GIT_FACTS: registration binding missing")?;
    let second = source
        .as_object_mut()
        .ok_or("E_GIT_FACTS: source binding missing")?;
    let extra = second
        .remove("processes")
        .and_then(|v| v.as_array().cloned())
        .ok_or("E_GIT_FACTS: source processes missing")?;
    if first.len() != second.len() + 1
        || !second
            .iter()
            .all(|(key, value)| first.get(key) == Some(value))
    {
        return Err("E_GIT_FACTS: registration and source bindings differ".into());
    }
    first
        .get_mut("processes")
        .and_then(Value::as_array_mut)
        .ok_or("E_GIT_FACTS: registration processes missing")?
        .extend(extra);
    Ok(())
}
