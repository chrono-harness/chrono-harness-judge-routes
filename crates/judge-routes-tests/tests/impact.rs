#[path = "../../judge-routes/src/impact.rs"]
mod impact;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn filemap_validates_the_original_document_and_supplies_route_inputs() {
    let input = json!({"workflow":{},"closure":{"reached":{}}});
    let env = BTreeMap::from([("PATH".into(), "/installed".into())]);
    let protocol = json!({"timeout_seconds":13,"stdout_limit_bytes":4096});
    let summary=impact::decode(&input,&protocol,&env,|cmd,bytes| {
        assert_eq!(cmd.program,"chrono-judge-filemap");
        assert_eq!(cmd.args,["impact-summary"]);
        assert_eq!(cmd.env,env);
        assert_eq!((cmd.timeout_seconds,cmd.output_limit_bytes),(13,4096));
        assert_eq!(serde_json::from_slice::<Value>(bytes).unwrap(),input);
        Ok((0,serde_json::to_vec(&json!({"schema":"chrono-filemap-impact/v6","delta":[],"reached":["file:message.py"],"tests":["test:message"],"retired_tests":["test:old"]})).unwrap(),vec![]))
    }).unwrap();
    assert_eq!(summary.schema, "chrono-filemap-impact/v6");
    assert!(summary.delta.is_empty());
    assert_eq!(summary.reached, ["file:message.py"]);
    assert_eq!(summary.tests, ["test:message"]);
    assert_eq!(summary.retired_tests, ["test:old"]);
    for (outcome, expected) in [
        (
            Ok((2, vec![], b"bad edge\n".to_vec())),
            "E_IMPACT: bad edge",
        ),
        (Err("timed out".into()), "E_IMPACT: timed out"),
        (Ok((0, b"{}".to_vec(), vec![])), "E_IMPACT: missing field"),
    ] {
        assert!(
            impact::decode(&input, &protocol, &env, |_, _| outcome)
                .unwrap_err()
                .starts_with(expected)
        );
    }
}
