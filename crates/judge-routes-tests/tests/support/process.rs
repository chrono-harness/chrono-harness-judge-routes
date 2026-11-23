//! Produce fixture receipts using only the test product's own Rust tool double.
use chrono_judge_routes::{ProcessResult, digest, protocol::CommandSpec, sha256};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn resolve(root: &Path, program: &str, path: &str) -> Result<PathBuf, String> {
    if program.contains('/') {
        return Ok(root.join(program));
    }
    std::env::split_paths(path)
        .map(|dir| root.join(dir).join(program))
        .find(|p| p.is_file())
        .ok_or_else(|| "missing mock program".into())
}

pub fn run_process_observed(
    root: &Path,
    spec: &CommandSpec,
    input: &[u8],
    expected: &str,
) -> Result<ProcessResult, String> {
    let path = resolve(
        root,
        &spec.program,
        spec.env.get("PATH").map(String::as_str).unwrap_or(""),
    )?;
    let identity = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
    if identity != expected {
        return Err("executable digest mismatch".into());
    }
    assert!(input.is_empty());
    // All supported fixture commands map to the known test binary. Never execute the supplied path.
    let fixture = std::env::current_exe().map_err(|e| e.to_string())?;
    let parent = fixture.parent().unwrap();
    let bin = if parent.file_name().is_some_and(|n| n == "deps") {
        parent.parent().unwrap()
    } else {
        parent
    };
    let out = Command::new(bin.join("chrono-judge-routes-test-tool"))
        .args(&spec.args)
        .current_dir(root)
        .env_clear()
        .envs(&spec.env)
        .output()
        .map_err(|e| e.to_string())?;
    let mut argv = vec![path.to_str().unwrap().to_owned()];
    argv.extend(spec.args.clone());
    serde_json::from_value(json!({"argv":argv,"environment":spec.env,"environment_digest":digest(&spec.env)?,"cwd":root,
     "executable":path,"sha256":identity,"stdin_sha256":sha256(input),"exit_code":out.status.code().unwrap_or(-1),
     "stdout_bytes":out.stdout,"stderr_bytes":out.stderr,"failure":null,"stdout_complete":true,"stderr_complete":true,
     "stdout_sha256":sha256(&out.stdout),"stderr_sha256":sha256(&out.stderr),"stdout":String::from_utf8_lossy(&out.stdout),
     "stderr":String::from_utf8_lossy(&out.stderr),"owned_work_joined":true}))
     .map_err(|e|e.to_string())
}
