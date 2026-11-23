//! Project route declarations onto the verifier's fixed-endpoint coverage query.
use crate::{Method, Operation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, path::Path};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Endpoint {
    pub commit: String,
    pub filemap: Value,
    pub projects: Value,
    pub artifacts: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitBinding {
    config: String,
    git_facts: Value,
}
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
    let reply: Reply = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if reply.candidate != candidate.commit || reply.previous.as_ref() != previous.map(|e| &e.commit)
    {
        return Err("E_GIT_FACTS: source coverage endpoint identity mismatch".into());
    }
    serde_json::to_value(reply).map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    previous: Option<String>,
    candidate: String,
    git_facts: Value,
    error: Option<String>,
}

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
