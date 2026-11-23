use crate::protocol::{CommandSpec, Delta};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Deserialize)]
pub(crate) struct ImpactSummary {
    pub schema: String,
    pub delta: Vec<Delta>,
    pub reached: Vec<String>,
    pub tests: Vec<String>,
    pub retired_tests: Vec<String>,
}

pub(crate) fn decode(
    impact: &Value,
    protocol: &Value,
    environment: &BTreeMap<String, String>,
    run: impl FnOnce(&CommandSpec, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<ImpactSummary, String> {
    let command = CommandSpec {
        program: "chrono-judge-filemap".into(),
        args: vec!["impact-summary".into()],
        env: environment.clone(),
        timeout_seconds: protocol["timeout_seconds"]
            .as_u64()
            .ok_or("invalid protocol timeout")?,
        output_limit_bytes: protocol["stdout_limit_bytes"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("invalid protocol output bound")?,
    };
    let input = serde_json::to_vec(impact).map_err(|error| format!("E_IMPACT: {error}"))?;
    let (exit_code, stdout, stderr) =
        run(&command, &input).map_err(|error| format!("E_IMPACT: {error}"))?;
    if exit_code != 0 {
        let diagnostic =
            std::str::from_utf8(&stderr).map_err(|error| format!("E_IMPACT: {error}"))?;
        let diagnostic = diagnostic.strip_suffix('\n').unwrap_or(diagnostic);
        return Err(if diagnostic.is_empty() {
            format!("E_IMPACT: FILEMAP impact-summary exited with {}", exit_code)
        } else {
            format!("E_IMPACT: {diagnostic}")
        });
    }
    serde_json::from_slice(&stdout).map_err(|error| format!("E_IMPACT: {error}"))
}
