//! The process owner reads the command descriptor and transports the original byte streams.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Each stream has its own budget. The payload must contain both in full.
pub fn valid_stream_lengths(stdout: usize, stderr: usize, payload: usize, limit: usize)
    -> (valid: bool)
    ensures valid == (stdout <= limit && stderr <= limit && stdout as int + stderr as int == payload as int),
{
    stdout <= limit && stderr <= limit && stdout <= payload && stderr == payload - stdout
}

}

#[cfg(not(verus_keep_ghost))]
use crate::protocol::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use crate::verified_admission::{RunnerError, runner_response};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

/// Keep the search path's order across independent tool installations. A host
/// can also launch through explicit sibling aliases that it does not export.
#[cfg(not(verus_keep_ghost))]
pub fn runner_program() -> std::path::PathBuf {
    let name = "chrono-harness";
    if std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
    }) {
        return name.into();
    }
    let invoked = std::env::args_os().next().map(std::path::PathBuf::from);
    let alias = invoked
        .as_deref()
        .and_then(Path::parent)
        .filter(|directory| !directory.as_os_str().is_empty())
        .and_then(|directory| std::path::absolute(directory).ok())
        .map(|directory| directory.join(name));
    alias.unwrap_or_else(|| name.into())
}

#[cfg(not(verus_keep_ghost))]
fn call(
    root: Option<&Path>,
    operation: &str,
    send: impl FnOnce(std::process::ChildStdin) -> Result<(), String> + Send,
) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new(runner_program());
    if let Some(root) = root {
        cmd.current_dir(root);
    }
    let mut child = cmd
        .arg(operation)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("start chrono-harness {operation}: {e}"))?;
    let stdin = child.stdin.take().unwrap();
    let (output, sent) = std::thread::scope(|scope| {
        let sender = scope.spawn(|| send(stdin));
        (child.wait_with_output(), sender.join())
    });
    let out = output.map_err(|e| e.to_string())?;
    let sent = sent
        .map_err(|_| "runner request writer panicked".to_owned())
        .and_then(|result| result);
    runner_response(out.status.success(), out.stdout, out.stderr, sent).map_err(|failure| {
        match failure {
            RunnerError::ExitStatus => format!("chrono-harness {operation} {}", out.status),
            RunnerError::Request(error) => error,
            RunnerError::Diagnostic(bytes) => match String::from_utf8(bytes) {
                Ok(mut reason) => {
                    if reason.ends_with('\n') {
                        reason.pop();
                    }
                    reason
                }
                Err(error) => error.to_string(),
            },
        }
    })
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn query(operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    call(None, operation, |stdin| {
        serde_json::to_writer(stdin, input).map_err(|e| e.to_string())
    })
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn run(
    root: &Path,
    command: &CommandSpec,
    input: &[u8],
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    #[derive(Deserialize)]
    struct Header {
        exit_code: i32,
        stdout_length: usize,
        stderr_length: usize,
    }
    let output = call(Some(root), "run-command-v1", |mut stdin| {
        serde_json::to_writer(&mut stdin, command).map_err(|e| e.to_string())?;
        stdin
            .write_all(b"\n")
            .and_then(|_| stdin.write_all(input))
            .map_err(|e| e.to_string())
    })?;
    let newline = output
        .iter()
        .position(|b| *b == b'\n')
        .ok_or("E_PROCESS: missing stream lengths")?;
    let lengths: Header = serde_json::from_slice(&output[..newline]).map_err(|e| e.to_string())?;
    let data = &output[newline + 1..];
    if !valid_stream_lengths(
        lengths.stdout_length,
        lengths.stderr_length,
        data.len(),
        command.output_limit_bytes,
    ) {
        return Err("E_PROCESS: invalid stream lengths".into());
    }
    let (stdout, stderr) = data.split_at(lengths.stdout_length);
    Ok((lengths.exit_code, stdout.to_vec(), stderr.to_vec()))
}
