//! Route-owned fields of the public judge request. Forward opaque evidence intact.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

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
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub commit: String,
    pub tree: String,
    pub root: PathBuf,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Registries {
    base: PathBuf,
    candidate: PathBuf,
    pub digest: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    path: PathBuf,
    pub sha256: String,
}
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
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
