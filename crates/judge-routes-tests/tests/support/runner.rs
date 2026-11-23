//! Mock only the installed runner calls made by route unit tests.
mod process;
use chrono_judge_routes::{protocol::CommandSpec, sha256};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    root: PathBuf,
    program: String,
    version_argv: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
}
fn query(op: &str, input: Value) -> Result<Value, String> {
    match op {
        "source-coverage-v1" => {
            let candidate = &input["candidate"]["commit"];
            if candidate != &Value::String("a".repeat(40)) {
                return Err("source commit is not the fixture revision".into());
            }
            assert_eq!(input["affected"], json!([]));
            assert_eq!(input["invocations"], json!([]));
            assert_eq!(input["scopes"], json!([]));
            let previous = &input["previous"];
            let error = if previous.is_null() {
                Value::Null
            } else {
                assert_eq!(previous["filemap"]["files"], false);
                json!("E_VERUS_COVERAGE: base file classifications unavailable: malformed fixture")
            };
            Ok(
                json!({"previous":previous["commit"],"candidate":candidate,"error":error,"git_facts":null}),
            )
        }
        "observed-tool-v1" => {
            let q: Observation = serde_json::from_value(input).map_err(|e| e.to_string())?;
            let path = process::resolve(
                &q.root,
                &q.program,
                q.environment.get("PATH").map(String::as_str).unwrap_or(""),
            )?;
            let hash = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
            let spec = CommandSpec {
                program: path.to_string_lossy().into(),
                args: q.version_argv,
                env: q.environment,
                timeout_seconds: q.timeout_seconds,
                output_limit_bytes: q.output_limit_bytes,
            };
            let version = process::run_process_observed(&q.root, &spec, &[], &hash)?;
            Ok(
                json!({"program":q.program,"path":path,"basename":path.file_name().unwrap().to_str().unwrap(),"sha256":hash,"version":version}),
            )
        }
        "compiler-output-v1" if input["operation"] == "check" => {
            let expected = json!({"path":b"library out.rlib".to_vec(),"sha256":sha256(&vec![255;65537]).into_bytes(),"length":65537});
            assert_eq!(
                input["contract"],
                json!({"version":"fixture","commit":"fixture"})
            );
            assert_eq!(
                input["dependency"],
                json!({"Ok":b"original dependency bytes".to_vec()})
            );
            assert_eq!(input["exit"], 0);
            assert_eq!(input["process_failed"], false);
            assert_eq!(input["limit"], 4096);
            assert_eq!(input["argv"][10], "library out.rlib");
            let bytes: Vec<u8> = serde_json::from_value(input["stdout"].clone()).unwrap();
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(report["verus"]["version"], "fixture");
            if input["artifact"] == json!({"Ok":expected}) {
                Ok(Value::Null)
            } else {
                Err(
                    "E_VERUS_ARTIFACT: fixture does not match original accepted by compiler owner"
                        .into(),
                )
            }
        }
        "check-invocation-v1" if input["mode"] == "template" => {
            let root = Path::new(input["root"].as_str().unwrap());
            let declared = json!([
                "runner",
                "check",
                "--config",
                "config",
                "--base",
                "{base}",
                "--candidate",
                "{candidate}"
            ]);
            assert_eq!(input["argv"], declared);
            let wanted = json!([
                root.join("runner"),
                "check",
                "--config",
                root.join("config"),
                "--base",
                input["binding"]["base"],
                "--candidate",
                input["binding"]["candidate"]
            ]);
            if input["entry"]["argv"] == wanted {
                Ok(Value::Null)
            } else {
                Err("invocation does not match fixture".into())
            }
        }
        _ => Err(format!("unexpected runner fixture operation {op}: {input}")),
    }
}
fn main() {
    let input = serde_json::from_reader(std::io::stdin().lock()).unwrap();
    match query(&std::env::args().nth(1).unwrap(), input) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2)
        }
    }
}
