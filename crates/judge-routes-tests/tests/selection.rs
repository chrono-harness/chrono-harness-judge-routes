use chrono_judge_routes::selection::select_from_registries;
use serde_json::{Value, json};

fn query() -> Value {
    let registries: Value =
        serde_json::from_str(include_str!("fixtures/selection-registries.json")).unwrap();
    json!({"config":".chrono-harness/config.json","base":registries,"candidate":registries,
        "tests":["test:example","test:example"],"retired_tests":[]})
}
fn select(q: &Value) -> Result<Value, String> {
    let mut output = Vec::new();
    select_from_registries(serde_json::to_vec(q).unwrap().as_slice(), &mut output)?;
    Ok(serde_json::from_slice(&output).unwrap())
}

#[test]
fn selects_unique_tests_and_rejects_an_unretired_baseline_identity() {
    let mut q = query();
    assert_eq!(select(&q).unwrap(), json!(["test:example"]));
    q["retired_tests"] = json!(["test:example"]);
    assert!(
        select(&q)
            .unwrap_err()
            .starts_with("E_RETIREMENT: test is not a removed baseline identity")
    );
}

#[test]
fn selects_declared_replacement_and_defers_joint_removal_only_to_a_downstream_validator() {
    let mut q = query();
    q["tests"] = json!([]);
    q["retired_tests"] = json!(["test:example"]);
    let projects = ".chrono-harness/projects.json";
    let workflow = ".chrono-harness/workflow.json";
    q["candidate"][projects]["scripts"][1]["id"] = json!("replacement");
    q["candidate"][projects]["scripts"][0]["test_script"] = json!("replacement");
    q["candidate"][workflow]["retirements"] = json!([{"kind":"test","id":"example","reason":"Move example coverage","replacement":"replacement"}]);
    assert_eq!(select(&q).unwrap(), json!(["test:replacement"]));
    q["candidate"][projects]["scripts"] = json!([]);
    q["candidate"][workflow]["retirements"] = json!([{"kind":"test","id":"example","reason":"Move example coverage","replacement":null},{"kind":"script","id":"message","reason":"Remove example producer","replacement":null}]);
    let judges = ".chrono-harness/judges.json";
    q["candidate"][judges]["migration_validator"] = json!("projects");
    assert_eq!(select(&q).unwrap(), json!([]));
    q["candidate"][judges]["migration_validator"] = json!("routes");
    assert_eq!(
        select(&q).unwrap_err(),
        "E_REQUIRED_TEST_REMOVED: test:example"
    );
}
