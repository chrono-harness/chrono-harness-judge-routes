use chrono_harness::{CommandSpec, run_process_observed, sha256, wire};
use chrono_judge_routes::{Receipt, compare, order, prepare};
use serde_json::json;
use std::{collections::BTreeMap, fs, os::unix::fs::symlink};

const TOOL: &str = env!("CARGO_BIN_EXE_chrono-judge-routes-test-tool");

// Only registration interprets project definitions. Supply its public response
// at the command boundary and assert both original documents reach it.
fn registry_reply<'a>(
    input: &'a serde_json::Value,
    methods: serde_json::Value,
) -> impl FnOnce(&str, &serde_json::Value) -> Result<Vec<u8>, String> + 'a {
    move |operation, request| {
        assert_eq!(operation, "execution-declarations-v1");
        assert_eq!(
            request,
            &json!({"filemap":input["filemap"],"projects":input["projects"]})
        );
        Ok(
            json!({"plans":input["filemap"]["execution_plans"], "methods":{"Ok":methods}})
                .to_string()
                .into_bytes(),
        )
    }
}

fn declarations() -> serde_json::Value {
    json!([{"id":"routes-helper","program":TOOL,"version_argv":["--version"],"expected_version":"chrono-judge-routes-test-tool 0.1.0"}])
}
fn method<T: serde::de::DeserializeOwned>(id: &str, args: Vec<String>) -> T {
    serde_json::from_value(
        json!({"owner":"script:p","operation":id,"tool":"routes-helper","argv":args}),
    )
    .unwrap()
}
fn plan<T: serde::de::DeserializeOwned>(operations: &[&str]) -> T {
    serde_json::from_value(json!({
        "operations":operations, "timeout_seconds":10, "output_limit_bytes":4096
    }))
    .unwrap()
}

// Supply the public receipt a project executor would send. File observation belongs to projects.
fn original_receipt(
    plan: &chrono_judge_routes::Execution,
    op: &chrono_judge_routes::Operation,
    process: chrono_harness::ProcessResult,
) -> Receipt {
    let mut receipt = Receipt {
        plan: plan.identity.clone(),
        operation: op.method.operation.clone(),
        method: wire::digest(&op.method).unwrap(),
        process,
        compiler_dependency: None,
        compiler_artifact: None,
        digest: String::new(),
    };
    seal(&mut receipt);
    receipt
}

fn seal(receipt: &mut Receipt) {
    let mut fields = serde_json::to_value(&*receipt).unwrap();
    fields.as_object_mut().unwrap().remove("digest");
    receipt.digest = wire::digest(&fields).unwrap();
}

#[test]
fn compiler_receipts_are_checked_offline_with_the_original_io_outcome() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let args: Vec<String> = [
        "--crate-type=lib",
        "--no-cheating",
        "--no-external-by-default",
        "--num-threads",
        "2",
        "--output-json",
        "--compile",
        "--emit=link=library out.rlib,dep-info=library out.rlib.d",
        "root.rs",
        "-o",
        "library out.rlib",
        "-Zchecksum-hash-algorithm=sha256",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut compiler: serde_json::Value = method("compile-fixture", args);
    compiler["output_contract"] = json!({"version":"fixture","commit":"fixture"});
    let plans = BTreeMap::from([("test:compiler".into(), plan(&["compile-fixture"]))]);
    let methods = serde_json::from_value(json!({"compile-fixture":[compiler]})).unwrap();
    let exec = BTreeMap::from([("test:compiler".into(), "compile-fixture".into())]);
    let plan = prepare(
        root,
        json!({}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &exec,
        &declarations(),
        BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    let op = &plan.operations[0];
    let tool = &plan.tools["routes-helper"];
    let command = CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: op.method.argv.clone(),
        env: plan.environment.clone(),
        timeout_seconds: 10,
        output_limit_bytes: 4096,
    };
    let process = run_process_observed(root, &command, &[], &tool.sha256).unwrap();
    let mut retained = original_receipt(&plan, op, process);
    retained.compiler_dependency = Some(Ok(b"original dependency bytes".to_vec()));
    retained.compiler_artifact = Some(Err("original artifact was missing".into()));
    seal(&mut retained);
    assert!(
        compare(&plan, op, Some(&retained))
            .unwrap_err()
            .starts_with("E_VERUS_ARTIFACT:")
    );
    let bytes = vec![255u8; 65537];
    retained.compiler_artifact = Some(Ok(chrono_harness::verified_verifier::CompiledArtifact {
        path: b"library out.rlib".to_vec(),
        sha256: sha256(&bytes).into_bytes(),
        length: bytes.len() as u64,
    }));
    seal(&mut retained);
    assert!(!root.join("library out.rlib").exists());
    compare(&plan, op, Some(&retained)).unwrap();

    for field in ["path", "sha256", "length"] {
        let mut changed = retained.clone();
        let artifact = changed
            .compiler_artifact
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap();
        match field {
            "path" => artifact.path.push(0),
            "sha256" => artifact.sha256 = b"invalid".to_vec(),
            _ => artifact.length = 0,
        }
        let mut value = serde_json::to_value(&changed).unwrap();
        value.as_object_mut().unwrap().remove("digest");
        changed.digest = wire::digest(&value).unwrap();
        assert!(
            compare(&plan, op, Some(&changed))
                .unwrap_err()
                .starts_with("E_VERUS_ARTIFACT:")
        );
    }
}
#[test]
fn shared_prerequisites_order_and_dedup() {
    let plans = BTreeMap::from([
        ("test:a".into(), plan(&["shared", "a"])),
        ("test:b".into(), plan(&["shared", "a", "b"])),
    ]);
    let methods = ["shared", "a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let (_, ops) = order(&plans.keys().cloned().collect(), &plans, &methods, &exec).unwrap();
    assert_eq!(
        ops.iter()
            .map(|o| o.method.operation.as_str())
            .collect::<Vec<_>>(),
        ["shared", "a", "b"]
    );
}
#[test]
fn explicit_prerequisite_bounds_preserve_each_consumer_default_and_reject_disagreement() {
    let mut plans: BTreeMap<String, _> = serde_json::from_value(json!({
        "test:distribution": {"operations":["build", "distribution"],
            "timeout_seconds":600,"output_limit_bytes":1048576},
        "test:startup": {"operations":["build", "startup"],
            "timeout_seconds":900,"output_limit_bytes":1048576,
            "operation_bounds":{"build":{"timeout_seconds":600,"output_limit_bytes":1048576}}}
    }))
    .unwrap();
    let methods = ["build", "distribution", "startup"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([
        ("test:distribution".into(), "distribution".into()),
        ("test:startup".into(), "startup".into()),
    ]);
    let selected = plans.keys().cloned().collect();
    let (chosen, ops) = order(&selected, &plans, &methods, &exec).unwrap();
    assert_eq!(chosen["test:startup"].timeout_seconds, 900);
    assert_eq!(
        ops.iter()
            .map(|o| (
                o.method.operation.as_str(),
                o.timeout_seconds,
                o.output_limit_bytes
            ))
            .collect::<Vec<_>>(),
        [
            ("build", 600, 1048576),
            ("distribution", 600, 1048576),
            ("startup", 900, 1048576)
        ]
    );
    assert_eq!(ops[2].predecessors, ["build".into()].into());
    let directory = tempfile::tempdir().unwrap();
    let collection = chrono_judge_routes::prepare_collection(
        directory.path(),
        json!({}),
        &selected,
        &plans,
        &methods,
        &exec,
        BTreeMap::new(),
    )
    .unwrap();
    assert!(collection.tools.is_empty());
    assert_eq!(collection.selected, chosen);
    assert_eq!(
        serde_json::to_value(&collection.operations).unwrap(),
        serde_json::to_value(&ops).unwrap()
    );
    let decoded: chrono_judge_routes::Execution =
        serde_json::from_value(serde_json::to_value(collection).unwrap()).unwrap();
    decoded.validate().unwrap();
    let original = serde_json::to_value(&plans).unwrap();
    for bounds in [
        json!({"timeout_seconds":601,"output_limit_bytes":1048576}),
        json!({"timeout_seconds":600,"output_limit_bytes":1048577}),
    ] {
        let mut conflicting = original.clone();
        conflicting["test:startup"]["operation_bounds"]["build"] = bounds;
        let conflicting = serde_json::from_value(conflicting).unwrap();
        assert!(
            order(&selected, &conflicting, &methods, &exec)
                .unwrap_err()
                .contains("E_PLAN_BOUNDS")
        );
    }
    plans.get_mut("test:startup").unwrap().timeout_seconds = 901;
    let (_, ops) = order(&selected, &plans, &methods, &exec).unwrap();
    assert_eq!(ops[0].timeout_seconds, 600);
    assert_eq!(ops[2].timeout_seconds, 901);
}

#[test]
fn explicit_shared_prerequisite_keeps_registered_test_contracts() {
    let mut query: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shared-plans.json")).unwrap();
    let startup = "test:host-bootstrap-native-tests";
    let plans = &mut query["filemap"]["execution_plans"];
    let original_operations: Vec<String> =
        serde_json::from_value(plans[startup]["operations"].clone()).unwrap();
    assert!(
        original_operations
            .iter()
            .any(|op| op == "test.host-bootstrap-native-tests")
    );
    assert!(plans[startup].get("operation_bounds").is_none());
    plans[startup]["operations"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!("build.cache"));
    plans[startup]["operation_bounds"] = json!({"build.cache":{
        "timeout_seconds":600, "output_limit_bytes":1048576
    }});
    let query_routes = |input: &serde_json::Value| {
        chrono_judge_routes::plan_query::full_operations_with(
            input.to_string().as_bytes(),
            registry_reply(
                input,
                serde_json::from_str(include_str!("fixtures/shared-methods.json")).unwrap(),
            ),
        )
    };
    let result = query_routes(&query).unwrap();
    let operations: Vec<chrono_judge_routes::Operation> =
        serde_json::from_value(result["operations"].clone()).unwrap();
    assert_eq!(
        operations
            .iter()
            .filter(|o| o.method.operation == "build.cache")
            .count(),
        1
    );
    for (operation, timeout) in [
        ("build.cache", 600),
        ("test.distribution-tests", 600),
        ("test.host-release-native-tests", 600),
    ]
    .into_iter()
    .chain(original_operations.iter().map(|op| (op.as_str(), 900)))
    {
        let op = operations
            .iter()
            .find(|o| o.method.operation == operation)
            .unwrap();
        assert_eq!(op.timeout_seconds, timeout, "{operation}");
        assert_eq!(op.output_limit_bytes, 1048576, "{operation}");
    }
    let selected = &result["selected"];
    let expected_operations: Vec<_> = std::iter::once("build.cache".to_owned())
        .chain(original_operations)
        .collect();
    assert_eq!(selected[startup]["operations"], json!(expected_operations));
    assert_eq!(selected[startup]["timeout_seconds"], 900);
    for test in ["test:distribution-tests", "test:host-release-native-tests"] {
        assert_eq!(selected[test]["timeout_seconds"], 600);
        assert!(selected[test].get("operation_bounds").is_none());
    }
    query["filemap"]["execution_plans"][startup]
        .as_object_mut()
        .unwrap()
        .remove("operation_bounds");
    assert!(
        query_routes(&query)
            .unwrap_err()
            .contains("E_PLAN_BOUNDS: inconsistent shared operation build.cache")
    );
}
#[test]
fn conflicts_missing_duplicate_methods_and_inconsistent_bounds_preflight() {
    let mut plans = BTreeMap::from([
        ("test:a".into(), plan(&["a", "b"])),
        ("test:b".into(), plan(&["b", "a"])),
    ]);
    let mut methods: BTreeMap<_, _> = ["a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let selected = plans.keys().cloned().collect();
    // A nonexistent tool path demonstrates that cycle rejection precedes tool processes.
    assert!(
        prepare(
            std::path::Path::new("/"),
            json!({}),
            &selected,
            &plans,
            &methods,
            &exec,
            &json!([]),
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("E_PLAN_CYCLE")
    );
    plans.get_mut("test:b").unwrap().operations = vec!["a".into(), "b".into()];
    plans.get_mut("test:b").unwrap().timeout_seconds = 20;
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_PLAN_BOUNDS")
    );
    plans.get_mut("test:b").unwrap().timeout_seconds = 10;
    methods.get_mut("a").unwrap().push(method("a", vec![]));
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_ROUTE_AMBIGUOUS")
    );
    methods.remove("a");
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_ROUTE_MISSING")
    );
}
#[test]
fn observed_literal_argv_environment_bytes_receipt_and_wrong_run() {
    let dir = tempfile::Builder::new()
        .prefix("routes host space ")
        .tempdir()
        .unwrap();
    let args = vec![
        "observe".into(),
        "$HOME `literal` <tag>".into(),
        "".into(),
        "line\nnext".into(),
    ];
    let methods = BTreeMap::from([("test".into(), vec![method("test", args.clone())])]);
    let plans = BTreeMap::from([("test:t".into(), plan(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    let plan = prepare(
        dir.path(),
        json!({"run":"one"}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &exec,
        &declarations(),
        BTreeMap::from([("EMPTY".into(), "".into())]),
        &BTreeMap::new(),
    )
    .unwrap();
    let op = &plan.operations[0];
    let tool = &plan.tools["routes-helper"];
    let process = run_process_observed(
        &plan.root,
        &CommandSpec {
            program: tool.path.to_str().unwrap().into(),
            args,
            env: plan.environment.clone(),
            timeout_seconds: 10,
            output_limit_bytes: 4096,
        },
        &[],
        &tool.sha256,
    )
    .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&process.stdout_bytes).unwrap();
    assert_eq!(process.exit_code, 0);
    assert_eq!(data[0], json!(["$HOME `literal` <tag>", "", "line\nnext"]));
    assert_eq!(data[1], json!(plan.root));
    assert_eq!(data[2], "");
    assert!(data[3].is_null());
    let receipt = original_receipt(&plan, op, process);
    let evidence = |plan: &serde_json::Value, receipts: serde_json::Value| {
        let bytes = serde_json::to_vec(&json!({"plan":plan,"receipts":receipts})).unwrap();
        chrono_judge_routes::plan_query::check_plan_receipts(bytes.as_slice())
    };
    let wire_plan = serde_json::to_value(&plan).unwrap();
    let name = &op.method.operation;
    let checked =
        chrono_judge_routes::plan_query::check_plan(wire_plan.to_string().as_bytes()).unwrap();
    assert_eq!(checked["plan"], plan.identity);
    let mut retained = json!({"tool":tool,"program":TOOL,"version_argv":["--version"],
        "environment":plan.environment,"root":plan.root,"expected_version":"chrono-judge-routes-test-tool 0.1.0"});
    let verify = |q: &serde_json::Value| {
        chrono_judge_routes::plan_query::retained_tool(q.to_string().as_bytes())
    };
    assert_eq!(verify(&retained).unwrap()["sha256"], tool.sha256);
    retained["root"] = json!("/another-host");
    assert!(verify(&retained).unwrap_err().contains("E_TOOL_BINDING"));
    let observed = evidence(&wire_plan, json!({ (name):receipt})).unwrap();
    assert_eq!(observed["plan"], plan.identity);
    assert_eq!(
        observed["operations"].get(name).unwrap(),
        &serde_json::Value::Null
    );
    let absent = evidence(&wire_plan, json!({})).unwrap();
    assert_eq!(
        absent["operations"][name],
        compare(&plan, op, None).unwrap_err()
    );
    let mut bad_receipt = serde_json::to_value(&receipt).unwrap();
    bad_receipt["digest"] = json!("0".repeat(64));
    let changed = evidence(&wire_plan, json!({(name):bad_receipt})).unwrap();
    assert!(
        changed["operations"][name]
            .as_str()
            .unwrap()
            .contains("E_RECEIPT_MISMATCH")
    );
    let mut bad_plan = wire_plan;
    bad_plan["identity"] = json!("0".repeat(64));
    assert_eq!(
        evidence(&bad_plan, json!({})).unwrap_err(),
        "E_PLAN_IDENTITY"
    );
    compare(&plan, op, Some(&receipt)).unwrap();
    assert!(compare(&plan, op, None).is_err());
    let mut tampered = receipt.clone();
    tampered.process.argv.push("injected".into());
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    let mut tampered = receipt.clone();
    tampered.plan = "other-run".into();
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    let mut tampered = receipt.clone();
    tampered.process.stdout_bytes.push(1);
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    for mode in 0..8 {
        let mut process = receipt.process.clone();
        match mode {
            0 => process.argv.push("different".into()),
            1 => process.cwd = "/".into(),
            2 => {
                process.environment.insert("NEW".into(), "input".into());
                process.environment_digest = wire::digest(&process.environment).unwrap();
            }
            3 => process.sha256 = "0".repeat(64),
            4 => process.stdin_sha256 = sha256(b"different input"),
            5 => process.stdout_sha256 = "0".repeat(64),
            6 => {
                process.stdout_bytes = vec![b'x'; op.output_limit_bytes + 1];
                process.stdout = String::from_utf8(process.stdout_bytes.clone()).unwrap();
                process.stdout_sha256 = sha256(&process.stdout_bytes);
            }
            _ => {
                process.stderr_bytes = vec![b'x'; op.output_limit_bytes + 1];
                process.stderr = String::from_utf8(process.stderr_bytes.clone()).unwrap();
                process.stderr_sha256 = sha256(&process.stderr_bytes);
            }
        }
        let sealed = original_receipt(&plan, op, process);
        assert!(
            compare(&plan, op, Some(&sealed)).is_err(),
            "resealed wrong observation mode {mode}"
        );
    }
    assert_eq!(
        receipt.process.stdout_sha256,
        sha256(&receipt.process.stdout_bytes)
    );
    assert_eq!(
        receipt.process.environment_digest,
        wire::digest(&plan.environment).unwrap()
    );
}
#[test]
fn tool_wrong_version_and_replacement_before_launch_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tool");
    symlink(TOOL, &path).unwrap();
    let mut tools = declarations();
    tools[0]["program"] = json!(path);
    tools[0]["expected_version"] = json!("wrong");
    let methods = BTreeMap::from([(
        "test".into(),
        vec![method("test", vec!["--version".into()])],
    )]);
    let plans = BTreeMap::from([("test:t".into(), plan(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    assert!(
        prepare(
            dir.path(),
            json!({}),
            &plans.keys().cloned().collect(),
            &plans,
            &methods,
            &exec,
            &tools,
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .is_err()
    );
    // Preserve a real executable identity; replacing its bytes must fail before spawn.
    let hash = sha256(&fs::read(&path).unwrap());
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"replaced").unwrap();
    assert!(
        run_process_observed(
            dir.path(),
            &CommandSpec {
                program: path.to_str().unwrap().into(),
                args: vec![],
                env: BTreeMap::new(),
                timeout_seconds: 10,
                output_limit_bytes: 4096
            },
            &[],
            &hash
        )
        .unwrap_err()
        .contains("digest mismatch")
    );
}

#[test]
fn path_shadow_cannot_replace_bound_invocation_and_bytes_are_rechecked() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let source = declarations();
    let shadow = dir.path().join("private shadow executable");
    let mut bytes = fs::read(TOOL).unwrap();
    bytes.extend_from_slice(b"\nprivate Rust command mock\n");
    fs::write(&shadow, bytes).unwrap();
    fs::set_permissions(&shadow, fs::metadata(TOOL).unwrap().permissions()).unwrap();
    symlink(TOOL, a.join("routes-helper")).unwrap();
    symlink(&shadow, b.join("routes-helper")).unwrap();
    let mut tools = source;
    tools[0]["program"] = json!("routes-helper");
    let methods = BTreeMap::from([("test".into(), vec![method("test", vec!["bound".into()])])]);
    let plans = BTreeMap::from([("test:t".into(), plan(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    let env = BTreeMap::from([("PATH".into(), a.to_str().unwrap().into())]);
    let plan = prepare(
        dir.path(),
        json!({}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &exec,
        &tools,
        env,
        &BTreeMap::new(),
    )
    .unwrap();
    let op = &plan.operations[0];
    let tool = &plan.tools["routes-helper"];
    let spec = CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: op.method.argv.clone(),
        env: BTreeMap::from([("PATH".into(), b.to_str().unwrap().into())]),
        timeout_seconds: 10,
        output_limit_bytes: 4096,
    };
    let actual = run_process_observed(dir.path(), &spec, &[], &tool.sha256).unwrap();
    assert_eq!(actual.exit_code, 0);
    assert_eq!(actual.stdout, "bound-rust\n");
    let receipt = original_receipt(&plan, op, actual);
    assert!(
        compare(&plan, op, Some(&receipt)).is_err(),
        "changed effective PATH cannot use old receipt binding"
    );
    fs::remove_file(a.join("routes-helper")).unwrap();
    symlink(&shadow, a.join("routes-helper")).unwrap();
    assert!(
        run_process_observed(dir.path(), &spec, &[], &tool.sha256)
            .unwrap_err()
            .contains("digest mismatch")
    );
}

#[test]
fn canonical_compares_actual_order_and_paths_instead_of_parsed_reconstruction() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    fs::write(root.join("runner"), b"runner").unwrap();
    fs::write(root.join("config"), b"config").unwrap();
    let template = vec![
        "runner".into(),
        "check".into(),
        "--config".into(),
        "config".into(),
        "--base".into(),
        "{base}".into(),
        "--candidate".into(),
        "{candidate}".into(),
    ];
    let argv = vec![
        root.join("runner").to_str().unwrap().to_string(),
        "check".into(),
        "--config".into(),
        root.join("config").to_str().unwrap().to_string(),
        "--base".into(),
        "a".into(),
        "--candidate".into(),
        "b".into(),
    ];
    let mut entry = json!({"argv":argv,"cwd":"/"});
    let binding = json!({"base":"a","candidate":"b"});
    chrono_judge_routes::validate_invocation(&root, &entry, &template, &binding).unwrap();
    entry["argv"] = json!([
        root.join("runner"),
        "check",
        "--base",
        "a",
        "--config",
        root.join("config"),
        "--candidate",
        "b"
    ]);
    assert!(
        chrono_judge_routes::validate_invocation(&root, &entry, &template, &binding).is_err(),
        "same parsed values with unregistered argv order must fail"
    );
}

#[test]
fn reused_tool_observation_avoids_second_version_launch_and_cycle_launches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let program = TOOL;
    let version_argv = vec!["count-version".into()];
    let environment = BTreeMap::new();
    let observed =
        chrono_harness::observation::tool(&root, program, &version_argv, &environment, 10, 4096)
            .unwrap();
    let declarations = json!([{"id":"routes-helper","program":program,"version_argv":version_argv,"expected_version":"chrono-judge-routes-test-tool 0.1.0"}]);
    let methods = ["a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec!["--version".into()])]))
        .collect();
    let mut plans = BTreeMap::from([
        ("test:a".into(), plan(&["a", "b"])),
        ("test:b".into(), plan(&["b"])),
    ]);
    let actions = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let selected = plans.keys().cloned().collect();
    let reused = BTreeMap::from([("routes-helper".into(), observed.clone())]);
    let plan = prepare(
        &root,
        json!({"run":"reuse"}),
        &selected,
        &plans,
        &methods,
        &actions,
        &declarations,
        environment.clone(),
        &reused,
    )
    .unwrap();
    assert_eq!(
        plan.tools["routes-helper"].version.stdout_bytes,
        observed.version.stdout_bytes
    );
    assert_eq!(fs::read_to_string(root.join("version-count")).unwrap(), "v");
    let mut wrong = reused;
    wrong
        .get_mut("routes-helper")
        .unwrap()
        .version
        .environment
        .insert("UNDECLARED".into(), "value".into());
    assert!(
        prepare(
            &root,
            json!({}),
            &selected,
            &plans,
            &methods,
            &actions,
            &declarations,
            environment.clone(),
            &wrong
        )
        .unwrap_err()
        .contains("E_TOOL_BINDING")
    );
    plans.get_mut("test:b").unwrap().operations = vec!["b".into(), "a".into()];
    assert!(
        prepare(
            &root,
            json!({}),
            &selected,
            &plans,
            &methods,
            &actions,
            &declarations,
            environment,
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("E_PLAN_CYCLE")
    );
    assert_eq!(
        fs::read_to_string(root.join("version-count")).unwrap(),
        "v",
        "conflicting plan launched no tool version process"
    );
}

#[test]
fn scheduling_binds_identity_and_collection_reconstructs_without_processes() {
    let d = tempfile::tempdir().unwrap();
    let plans = BTreeMap::from([("test:t".into(), plan(&["run"]))]);
    let methods = BTreeMap::from([("run".into(), vec![method("run", vec!["literal".into()])])]);
    let actions = BTreeMap::from([("test:t".into(), "run".into())]);
    let legacy = chrono_judge_routes::prepare_collection(
        d.path(),
        json!({}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &actions,
        BTreeMap::new(),
    )
    .unwrap();
    let value = serde_json::to_value(&legacy).unwrap();
    assert!(value.get("scheduling").is_none());
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("identity");
    assert_eq!(legacy.identity, wire::digest(&unsigned).unwrap());
    let policy = serde_json::from_value(
        json!({"max_running":2,"resources":[],"claims":{"run":{"resources":[],"outputs":[]}}}),
    )
    .unwrap();
    let concurrent = legacy.clone().with_scheduling(Some(policy)).unwrap();
    assert_ne!(legacy.identity, concurrent.identity);
    assert!(concurrent.tools.is_empty());
    let mut changed = serde_json::to_value(&concurrent).unwrap();
    changed["scheduling"]["max_running"] = json!(1);
    let changed: chrono_judge_routes::Execution = serde_json::from_value(changed).unwrap();
    assert!(changed.validate().is_err());
    let bad = serde_json::from_value(json!({"max_running":2,"resources":[],"claims":{}})).unwrap();
    assert!(legacy.with_scheduling(Some(bad)).is_err());
}

#[test]
fn collection_query_replays_only_the_unique_registered_terminal_operation() {
    let mut query = json!({"root":"/fixture","binding":{"candidate":"fixed"},
        "selected":["test:example"],"filemap":{"execution_plans":{"test:example":{
        "operations":["run.example"],"timeout_seconds":12,"output_limit_bytes":2048}}},
        "projects":{"projects":[],"scripts":[{"id":"example","actions":{"execute":{
        "operation":"run.example","tool":"python","argv":["{candidate}"]}}}]},
        "test_operations":{"test:example":["run.example"]},"environment":{"MODE":"offline"}});
    let replay = |data: &serde_json::Value| {
        chrono_judge_routes::plan_query::collection_plan_with(
            data.to_string().as_bytes(),
            registry_reply(
                data,
                json!({"run.example": [{"owner": "script:example", "operation": "run.example", "tool": "python", "argv": ["{candidate}"]}]}),
            ),
        )
    };
    let result = replay(&query).unwrap();
    assert_eq!(result["operations"][0]["method"]["argv"], json!(["fixed"]));
    assert_eq!(result["operations"][0]["timeout_seconds"], 12);
    assert_eq!(result["environment"], json!({"MODE":"offline"}));
    assert_eq!(result["tools"], json!({}));
    let plan: chrono_judge_routes::Execution = serde_json::from_value(result).unwrap();
    plan.validate().unwrap();
    query["test_operations"]["test:example"] = json!(["run.example", "run.other"]);
    assert!(replay(&query).is_err());
}

#[test]
fn unit_declarations_bind_only_owned_methods_to_the_original_candidate() {
    let end = json!({"commit":"candidate","tree":"tree","root":"/original"});
    let mut q = json!({
        "request":{"request_id":"fixed","protocol":"v1","judge_id":"routes","mode":"delta","base":end,
            "candidate":end,"delta":[],"registries":{"base":"/old","candidate":"/new","digest":"reg"},
            "context":{"path":"ctx","sha256":"ctx"},"impact":null,"prior_results":[],"config_path":"cfg",
            "checkout":{"head":"candidate","tracked":[],"untracked":[],"index_flags":[]},
            "runner":{"path":"runner","sha256":"runner","version":"1"}},
        "effective_inputs":{"id":"inputs"},
        "filemap":{"execution_plans":{}}, "projects":{"projects":[],"scripts":[]},
        "test_operations":{},"global":["test:a","test:b"],"unit":["test:b"]
    });
    for id in ["a", "b"] {
        let test = format!("test:{id}");
        q["filemap"]["execution_plans"][&test] =
            json!({"operations":[id],"timeout_seconds":9,"output_limit_bytes":128});
        q["test_operations"][&test] = json!([id]);
        q["projects"]["scripts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":id,
            "actions":{"execute":{"operation":id,"tool":"example","argv":["{candidate}"]}}}));
    }
    let read = |q: &serde_json::Value| {
        chrono_judge_routes::plan_query::unit_declarations_with(
            q.to_string().as_bytes(),
            registry_reply(
                q,
                json!({"a": [{"owner": "script:a", "operation": "a", "tool": "example", "argv": ["{candidate}"]}], "b": [{"owner": "script:b", "operation": "b", "tool": "example", "argv": ["{candidate}"]}]}),
            ),
        )
    };
    let declarations = read(&q).unwrap();
    assert_eq!(
        declarations["global_graph"]["operations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        declarations["global_graph"]["operations"][1]["method"]["argv"],
        json!(["{candidate}"])
    );
    assert_eq!(declarations["operations"].as_array().unwrap().len(), 1);
    assert_eq!(declarations["operations"][0]["method"]["operation"], "b");
    assert_eq!(
        declarations["operations"][0]["method"]["argv"],
        json!(["candidate"])
    );
    q["global"] = json!(["test:a"]);
    assert!(read(&q).unwrap_err().contains("E_UNIT_ASSIGNMENT"));
}

#[test]
fn scoped_graph_expands_only_when_bound_and_rejects_a_missing_action() {
    use chrono_judge_routes::plan_query::scoped_operations;
    let mut query = json!({"graph":{
        "selected":["test:a"], "plans":{"test:a":plan::<serde_json::Value>(&["first","last"])},
        "methods":{"first":[method::<serde_json::Value>("first",vec![])],"last":[method::<serde_json::Value>("last",vec!["{candidate}".into()])]},
        "execute":{"test:a":"last"}
    }});
    let read = |query: &serde_json::Value| scoped_operations(query.to_string().as_bytes());
    let raw = read(&query).unwrap();
    assert_eq!(raw["selected"], query["graph"]["plans"]);
    assert_eq!(raw["operations"][1]["predecessors"], json!(["first"]));
    assert_eq!(
        raw["operations"][1]["method"]["argv"],
        json!(["{candidate}"])
    );
    query["binding"] = json!({"candidate":"selected-revision"});
    let bound = read(&query).unwrap();
    assert_eq!(
        bound["operations"][1]["method"]["argv"],
        json!(["selected-revision"])
    );
    query["graph"]["execute"] = json!({});
    assert!(read(&query).unwrap_err().contains("E_ROUTE_MISSING"));
}

#[test]
fn scoped_preparation_binds_the_actual_tool_version_and_keeps_its_observation() {
    let host = tempfile::tempdir().unwrap();
    let mut query = json!({
        "graph":{"selected":["test:a"],"plans":{"test:a":plan::<serde_json::Value>(&["build"])},
                 "methods":{"build":[method::<serde_json::Value>("build",vec!["{candidate}".into()])]},
                 "execute":{"test:a":"build"}},
        "binding":{"candidate":"fixed"}, "declarations":declarations(),
        "environment":{}, "reused":{}, "scheduling":null,
    });
    let prepare = |query: &serde_json::Value| {
        chrono_judge_routes::plan_query::prepare_scoped_plan(
            host.path(),
            query.to_string().as_bytes(),
        )
    };
    let first = prepare(&query).unwrap();
    assert_eq!(first["operations"][0]["method"]["argv"], json!(["fixed"]));
    assert_eq!(first["tools"]["routes-helper"]["version"]["exit_code"], 0);
    query["reused"] = first["tools"].clone();
    assert_eq!(prepare(&query).unwrap(), first);
    query["declarations"][0]["expected_version"] = json!("different tool version");
    assert!(prepare(&query).is_err());
}

#[test]
fn full_scheduling_validates_even_unselected_assignments_and_collection_replays_the_order() {
    let mut query = json!({
        "selected":["test:a"],
        "filemap":{"execution_plans":{
            "test:a":plan::<serde_json::Value>(&["run.a"]), "test:b":plan::<serde_json::Value>(&["run.b"])
        }},
        "projects":{"projects":[], "scripts":[
            {"id":"a","actions":{"execute":{"operation":"run.a","tool":"a","argv":["{candidate}"]}}},
            {"id":"b","actions":{"execute":{"operation":"run.b","tool":"b","argv":[]}}}
        ]},
        "test_operations":{"test:a":["run.a"],"test:b":["run.b"]},
        "execution_units":{"units":{
            "alpha":{"tests":["test:a"],"report_path":"alpha.json"},
            "beta":{"tests":["test:b"],"report_path":"beta.json"}
        },"shared_operations":{}}
    });
    let read = |input: &serde_json::Value| {
        chrono_judge_routes::plan_query::full_operations_with(
            input.to_string().as_bytes(),
            registry_reply(
                input,
                json!({"run.a": [{"owner": "script:a", "operation": "run.a", "tool": "a", "argv": ["{candidate}"]}], "run.b": [{"owner": "script:b", "operation": "run.b", "tool": "b", "argv": []}]}),
            ),
        )
    };
    let scheduled = read(&query).unwrap();
    assert_eq!(scheduled["required_units"], json!(["alpha"]));
    assert_eq!(
        scheduled["operations"][0]["method"]["argv"],
        json!(["{candidate}"])
    );
    assert_eq!(scheduled["methods"]["run.b"][0]["owner"], "script:b");
    query["execution_units"]["units"]["beta"]["tests"] = json!(["test:missing"]);
    assert!(read(&query).unwrap_err().contains("E_UNIT_ASSIGNMENT"));
    query.as_object_mut().unwrap().remove("execution_units");
    let collected = read(&query).unwrap();
    assert_eq!(collected["required_units"], serde_json::Value::Null);
    for field in ["selected", "operations", "methods"] {
        assert_eq!(collected[field], scheduled[field]);
    }
    query["test_operations"]["test:a"] = json!(["run.a", "run.b"]);
    assert!(read(&query).unwrap_err().contains("E_ROUTE_MISSING"));
}

#[test]
fn source_plan_keeps_fixed_endpoint_identity_and_baseline_classification_errors() {
    let commit = "a".repeat(40);
    let endpoint = serde_json::json!({"commit":commit,"filemap":{"files":[],"require_new_file_classification":true},
        "projects":{"projects":[]}, "artifacts":[]});
    let mut query = serde_json::json!({"previous":null,"candidate":endpoint,"affected":[],
        "methods":{},"operations":[],"git":null});
    let read = |value: &serde_json::Value| {
        chrono_judge_routes::source_plan::query(
            std::path::Path::new("missing-checkout"),
            value.to_string().as_bytes(),
        )
    };
    assert_eq!(
        read(&query).unwrap(),
        serde_json::json!({"previous":null,"candidate":commit,"error":null,"git_facts":null})
    );
    let mut previous = endpoint;
    previous["filemap"]["files"] = serde_json::json!(false);
    query["previous"] = previous;
    let rejected = read(&query).unwrap();
    assert_eq!(rejected["previous"], commit);
    assert!(rejected["git_facts"].is_null());
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("E_VERUS_COVERAGE: base file classifications unavailable:")
    );
    query["candidate"]["commit"] = serde_json::json!("dev");
    assert!(read(&query).is_err());
}

#[test]
fn scoped_binding_uses_the_original_report_and_run_without_observing_tools() {
    let graph = json!({"selected":[],"plans":{},"methods":{},"execute":{}});
    let binding = json!({"run":"job-1","candidate":"fixed","entry":{"original":true}});
    let report = ".chrono-harness/state/unit/check.json";
    let input = json!({"graph":graph,"binding":binding,"report_path":report,"declarations":{},
        "environment":{},"reused":{},"scheduling":null});
    let root = tempfile::tempdir().unwrap();
    let prepare = |query: &serde_json::Value| {
        chrono_judge_routes::plan_query::prepare_scoped_plan(
            root.path(),
            query.to_string().as_bytes(),
        )
    };
    let plan = prepare(&input).unwrap();
    assert_eq!(
        plan["binding"]["operation_state"],
        chrono_judge_routes::operation_state_path(report, &binding["run"])
    );
    let original = json!({"plan":plan,"binding":binding,"report_path":report});
    let check = |q: &serde_json::Value| {
        chrono_judge_routes::plan_query::check_scoped_plan(q.to_string().as_bytes())
    };
    assert_eq!(check(&original).unwrap()["plan"], plan["identity"]);
    for (field, value) in [("run", json!("job-2")), ("entry", json!({}))] {
        let mut wrong = original.clone();
        wrong["binding"][field] = value;
        assert!(check(&wrong).unwrap_err().contains("E_PLAN_BINDING"));
    }
    let mut wrong = original.clone();
    wrong["report_path"] = json!(".chrono-harness/state/other/check.json");
    assert!(check(&wrong).unwrap_err().contains("E_PLAN_BINDING"));
    let mut supplied = input;
    supplied["binding"]["operation_state"] = plan["binding"]["operation_state"].clone();
    assert!(prepare(&supplied).unwrap_err().contains("E_PLAN_BINDING"));
}
