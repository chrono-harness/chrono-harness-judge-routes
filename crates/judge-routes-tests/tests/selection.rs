use chrono_judge_routes::selection::select_from_registries_with;
use serde_json::{Value, json};

fn select(tests: Value, retired: Value, reply: Value) -> Result<Value, String> {
    let docs: Value =
        serde_json::from_str(include_str!("fixtures/selection-registries.json")).unwrap();
    let q = json!({"config":".chrono-harness/config.json","base":docs,"candidate":docs,"tests":tests,"retired_tests":retired});
    let mut output = Vec::new();
    select_from_registries_with(q.to_string().as_bytes(), &mut output, |operation, input| {
        assert_eq!(operation, "test-retirements-v1");
        assert_eq!(
            input,
            &json!({"judge":"routes","endpoints":{"mode":"documents","config":".chrono-harness/config.json","base":docs,"candidate":docs},"retired_tests":retired})
        );
        Ok(serde_json::to_vec(&reply).unwrap())
    })?;
    Ok(serde_json::from_slice(&output).unwrap())
}

#[test]
fn deduplicates_selected_tests_and_preserves_first_retirement_error() {
    let tests = json!(["test:example", "test:example"]);
    let response = json!({"request_id":null,"downstream":false,"retirements":{}});
    assert_eq!(
        select(tests.clone(), json!([]), response.clone()).unwrap(),
        json!(["test:example"])
    );
    let mut response = response;
    response["retirements"] =
        json!({"test:b":{"Err":"original baseline error"},"test:a":{"Err":"later error"}});
    assert_eq!(
        select(tests, json!(["test:b", "test:a"]), response).unwrap_err(),
        "original baseline error"
    );
}

#[test]
fn selects_replacement_and_requires_downstream_validator_for_joint_removal() {
    let retired = json!(["test:old", "test:retired", "test:old"]);
    let mut response = json!({"request_id":null,"downstream":true,"retirements":{"test:old":{"Ok":"test:new"},"test:retired":{"Ok":null}}});
    assert_eq!(
        select(json!([]), retired.clone(), response.clone()).unwrap(),
        json!(["test:new"])
    );
    response["downstream"] = json!(false);
    assert_eq!(
        select(json!([]), retired.clone(), response.clone()).unwrap_err(),
        "E_REQUIRED_TEST_REMOVED: test:retired"
    );
    response["request_id"] = json!("different");
    assert!(
        select(json!([]), retired.clone(), response.clone())
            .unwrap_err()
            .contains("reply differs")
    );
    response["request_id"] = Value::Null;
    response["retirements"]
        .as_object_mut()
        .unwrap()
        .remove("test:old");
    assert!(
        select(json!([]), retired, response)
            .unwrap_err()
            .contains("reply differs")
    );
}

#[test]
fn rejects_extra_or_substituted_decisions_and_malformed_identity_before_selection() {
    let retired = json!(["test:old", "test:old"]);
    let valid =
        json!({"request_id":null, "downstream":true, "retirements":{"test:old":{"Ok":"test:new"}}});
    for (field, invalid) in [
        ("request_id", json!(42)),
        (
            "retirements",
            json!({"test:old":{"Ok":"test:new"}, "test:extra":{"Ok":null}}),
        ),
        ("retirements", json!({"test:other":{"Ok":"test:new"}})),
    ] {
        let mut reply = valid.clone();
        reply[field] = invalid;
        assert_eq!(
            select(json!([]), retired.clone(), reply).unwrap_err(),
            "E_RETIREMENT: registration reply differs from requested tests or collection"
        );
    }
}

#[test]
fn collection_forwards_the_sealed_request_and_unit_originals_and_binds_the_reply() {
    use chrono_judge_routes::selection::select_collected_with;
    use std::path::Path;
    let end = json!({"commit":"candidate","tree":"tree","root":"/selected"});
    let request = json!({"request_id":"fixed","protocol":"chrono-judge/v1","judge_id":"routes","mode":"evaluate",
        "base":end,"candidate":end,"delta":[],"registries":{"base":"/old","candidate":"/new","digest":"reg"},
        "context":{"path":"ctx","sha256":"ctx"},"impact":null,"prior_results":[],"config_path":"cfg",
        "checkout":{"head":"candidate","tracked":[],"untracked":[],"index_flags":[]},
        "runner":{"path":"runner","sha256":"runner","version":"1"},"observations":null,
        "scope":{"kind":"collect","manifest":"manifest.json"}});
    let migration =
        json!({"units":["fixture"],"originals":{"schema":"chrono-full-reports/v1","reports":[]}});
    let q =
        json!({"request":request,"tests":["test:kept"],"retired_tests":[],"migration":migration});
    let run = |root: &Path, returned: &str| {
        let mut output = Vec::new();
        select_collected_with(root, q.to_string().as_bytes(), &mut output, |op, input| {
            assert_eq!(op, "test-retirements-v1");
            assert_eq!(
                input["endpoints"],
                json!({"mode":"collection","request":request,"migration":migration})
            );
            Ok(
                json!({"request_id":returned,"downstream":false,"retirements":{}})
                    .to_string()
                    .into_bytes(),
            )
        })?;
        Ok::<Value, String>(serde_json::from_slice(&output).unwrap())
    };
    assert_eq!(
        run(Path::new("/selected"), "fixed").unwrap(),
        json!(["test:kept"])
    );
    assert!(
        run(Path::new("/selected"), "other")
            .unwrap_err()
            .contains("reply differs")
    );
    assert!(
        run(Path::new("/other"), "fixed")
            .unwrap_err()
            .contains("requires this checkout")
    );
}
