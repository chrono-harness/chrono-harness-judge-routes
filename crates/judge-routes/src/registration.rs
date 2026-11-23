//! Run a public query on the host-selected registration executable.
use crate::{Method, Plan, Scheduling};
use crate::{Tool, protocol::Request};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) fn query(root: &Path, operation: &str, query: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(
        "installed-query-v1",
        &json!({"root":root,"program":"chrono-judge-registration",
        "operation":operation,"environment":"search-path","input":query}),
    )
}

fn native_query(req: &Request, operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(
        "installed-query-v1",
        &json!({"root":req.candidate.root,"program":"chrono-judge-registration",
        "operation":operation,"environment":"inherited","input":input}),
    )
}

pub(super) fn runner(_root: &Path, operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    crate::commands::query(operation, input)
}

#[derive(Deserialize)]
pub(super) struct Endpoint {
    pub config: Value,
    pub filemap: Value,
    pub projects: Value,
}

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
    if reply.request_id != req.request_id
        || reply.commits != [req.base.commit.as_str(), req.candidate.commit.as_str()]
    {
        return Err("E_PROTOCOL: registration route views identity".into());
    }
    *git = reply.git_facts.unwrap_or(Value::Null);
    reply.result
}

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
    if reply.request_id != req.request_id {
        return Err("E_PROTOCOL: registration route inputs identity".into());
    }
    Ok(reply.effective_inputs)
}

/// Registration reads the fixed endpoints first. The route planner may read source continuity later.
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
