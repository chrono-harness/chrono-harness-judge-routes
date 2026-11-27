//! Route-owned fields of the public judge request. Forward opaque evidence intact.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Registration uses a null request ID only for unbound document queries.
pub fn request_matches(expected: Option<&String>, returned: Option<&String>) -> (matches: bool)
    ensures matches == match (expected, returned) {
        (None, None) => true,
        (Some(a), Some(b)) => a@ == b@,
        _ => false,
    },
{
    match (expected, returned) {
        (None, None) => true,
        (Some(a), Some(b)) => String::eq(a, b),
        _ => false,
    }
}
}

#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, path::PathBuf};

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Delta {
    kind: String,
    path: String,
    old_blob: Option<String>,
    new_blob: Option<String>,
    old_mode: Option<String>,
    new_mode: Option<String>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub commit: String,
    pub tree: String,
    pub root: PathBuf,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Registries {
    base: PathBuf,
    candidate: PathBuf,
    pub digest: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    path: PathBuf,
    pub sha256: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default)]
    pub observations: Value,
    protocol: String,
    pub request_id: String,
    pub judge_id: String,
    mode: String,
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub delta: Vec<Delta>,
    pub registries: Registries,
    pub context: Context,
    pub impact: Value,
    prior_results: Vec<Value>,
    pub config_path: String,
    checkout: Value,
    runner: Value,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}
#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
