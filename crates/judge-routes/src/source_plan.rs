//! Project route declarations onto the verifier's fixed-endpoint coverage query.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Reply<T> {
    pub previous: Option<String>,
    pub candidate: String,
    pub git_facts: T,
    pub error: Option<String>,
}

pub open spec fn same_previous(expected: Option<&str>, reported: Option<String>) -> bool {
    match (expected, reported) {
        (None, None) => true,
        (Some(a), Some(b)) => a@ == b@,
        _ => false,
    }
}

/// Admit the complete runner reply only when both requested endpoints match.
pub fn bind_coverage_reply<T>(previous: Option<&str>, candidate: &str, reply: Reply<T>)
    -> (result: Result<Reply<T>, &'static str>)
    ensures
        result.is_ok() == (candidate@ == reply.candidate@ && same_previous(previous, reply.previous)),
        result matches Ok(value) ==> value == reply,
        result matches Err(reason) ==> reason@ == "E_GIT_FACTS: source coverage endpoint identity mismatch"@,
{
    let matches_previous = match (previous, &reply.previous) {
        (None, None) => true,
        (Some(expected), Some(reported)) => <&str>::exec_eq(expected, reported.as_str()),
        _ => false,
    };
    if matches_previous && <&str>::exec_eq(candidate, reply.candidate.as_str()) {
        Ok(reply)
    } else {
        Err("E_GIT_FACTS: source coverage endpoint identity mismatch")
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{Method, Operation};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, io::Read, path::Path};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Endpoint {
    pub commit: String,
    pub filemap: Value,
    pub projects: Value,
    pub artifacts: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitBinding {
    config: String,
    git_facts: Value,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    previous: Option<Endpoint>,
    candidate: Endpoint,
    affected: Vec<String>,
    methods: BTreeMap<String, Vec<Method>>,
    operations: Vec<Operation>,
    git: Option<GitBinding>,
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn coverage(
    root: &Path,
    previous: Option<&Endpoint>,
    candidate: &Endpoint,
    affected: &[String],
    methods: &BTreeMap<String, Vec<Method>>,
    operations: &[Operation],
    git: Value,
) -> Result<Value, String> {
    let scopes: Vec<_> = methods
        .values()
        .flatten()
        .map(|m| {
            json!({
                "operation":m.operation,"owner":m.owner,"tool":m.tool
            })
        })
        .collect();
    let invocations: Vec<_> = operations.iter().map(|op| json!({
        "operation":op.method.operation,"argv":op.method.argv,"output_contract":op.method.output_contract,
        "output_limit_bytes":op.output_limit_bytes
    })).collect();
    let input = json!({"previous":previous, "candidate":candidate, "affected":affected,
        "scopes":scopes, "invocations":invocations, "git":git});
    let bytes = crate::registration::runner(root, "source-coverage-v1", &input)?;
    let reply: Reply<Value> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let checked = bind_coverage_reply(
        previous.map(|endpoint| endpoint.commit.as_str()),
        &candidate.commit,
        reply,
    )?;
    serde_json::to_value(checked).map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn query(root: &Path, input: impl Read) -> Result<Value, String> {
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let git = match q.git {
        None => json!({"mode":"legacy"}),
        Some(binding) => json!({"mode":"observed","config":binding.config,
            "observations":{"environment":binding.git_facts["environment"],"git_facts":binding.git_facts}}),
    };
    coverage(
        root,
        q.previous.as_ref(),
        &q.candidate,
        &q.affected,
        &q.methods,
        &q.operations,
        git,
    )
}
