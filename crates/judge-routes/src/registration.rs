//! Run a public query on the host-selected registration executable.
use serde_json::Value;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub(crate) fn query(root: &Path, operation: &str, query: &Value) -> Result<Vec<u8>, String> {
    let search = std::env::var("PATH").map_err(|e| e.to_string())?;
    let program =
        chrono_harness::resolve_program(root, "chrono-judge-registration", Some(&search))?;
    let digest = chrono_harness::file_identity(&program)?.0;
    let mut command = Command::new(&program);
    command
        .arg(operation)
        .current_dir(root)
        .env_clear()
        .env("PATH", search)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    chrono_harness::process_fds::forward_command(&mut command, &[])?;
    let mut child = command
        .spawn()
        .map_err(|e| format!("start {}: {e}", program.display()))?;
    let stdin = child.stdin.take().unwrap();
    let (output, sent) = std::thread::scope(|scope| {
        let sender = scope.spawn(|| serde_json::to_writer(stdin, query));
        (child.wait_with_output(), sender.join())
    });
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        let reason = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
        return Err(if reason.is_empty() {
            format!("registration {operation} {}", output.status)
        } else {
            reason.strip_suffix('\n').unwrap_or(&reason).into()
        });
    }
    sent.map_err(|_| "registration request writer panicked")?
        .map_err(|e| e.to_string())?;
    if chrono_harness::file_identity(&program)?.0 != digest {
        return Err("E_RETIREMENT: registration executable changed".into());
    }
    Ok(output.stdout)
}
