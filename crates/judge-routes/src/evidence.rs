//! JSON records returned by the installed process and compiler owners.
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub environment: std::collections::BTreeMap<String, String>,
    pub environment_digest: String,
    /// Actual process-engine capability carrier, separate from declared inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership_fds: Option<String>,
    pub stdin_sha256: String,
    pub stdout_bytes: Vec<u8>,
    pub stderr_bytes: Vec<u8>,
    #[serde(default)]
    pub stdout_complete: bool,
    #[serde(default)]
    pub stderr_complete: bool,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub failure: Option<String>,
    /// Structured failure evidence is additive to the historical string.  It
    /// is deliberately outside the judge-v1 request/response wire contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_record: Option<ErrorRecord>,
    pub exit_code: i32,
    /// Actual process-owner drain and direct join, independent of command success.
    #[serde(default)]
    pub owned_work_joined: bool,
    /// Actual terminal signal, including cancellation; exit_code keeps its wire meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub executable: PathBuf,
    pub sha256: String,
    /// Retained process records bind the launch identity to this receipt.
    /// These fields are optional only for historical inline records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "crate::declarations::timeout_wire::optional::deserialize")]
    pub timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_limit_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_original: Option<Value>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ErrorSchema {
    #[serde(rename = "chrono-error/v1")]
    V1,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorRecord {
    pub schema: ErrorSchema,
    pub error: ErrorNode,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorNode {
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: String,
    pub message: String,
    pub context: BTreeMap<String, Value>,
    pub source: Option<Box<ErrorNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<RelatedError>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedError {
    pub role: String,
    pub error: ErrorNode,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputContract {
    pub version: String,
    pub commit: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledArtifact {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: u64,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub program: String,
    pub path: PathBuf,
    pub basename: String,
    pub sha256: String,
    #[serde(serialize_with = "crate::declarations::timeout_wire::observation")]
    pub version: ProcessResult,
}

#[cfg(not(verus_keep_ghost))]
pub fn tool(
    root: &Path,
    program: &str,
    version_argv: &[String],
    environment: &BTreeMap<String, String>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
) -> Result<Tool, String> {
    let input = json!({"root":root,"program":program,"version_argv":version_argv,"environment":environment,"timeout_seconds":timeout_seconds,"output_limit_bytes":output_limit_bytes});
    let raw = crate::registration::runner(root, "observed-tool-v1", &input)?;
    serde_json::from_slice(&raw).map_err(|e| e.to_string())
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn compiler(input: Value) -> Result<Value, String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let raw = crate::registration::runner(&root, "compiler-output-v1", &input)?;
    serde_json::from_slice(&raw).map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// An absent timeout and a safely representable timeout retain their native JSON shape.
pub fn exact_observation_timeout(seconds: Option<u64>) -> (text: Option<u64>)
    ensures match seconds {
        Some(n) if n >= 9007199254740992 => text == Some(n),
        _ => text.is_none(),
    },
{
    match seconds { Some(n) if n >= 9007199254740992u64 => Some(n), _ => None }
}
}
