//! Independent lower and upper boundaries of an executable route operation.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::verified_bounds as policy;
#[cfg(verus_keep_ghost)]
#[path = "../../judge-routes/src/verified_bounds.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::{prelude::*, vpanic as panic};

verus! {

#[cfg(verus_keep_ghost)]
fn exact_timeout_edges() {
    let before = policy::timeout_bounds::decimal_timeout(9007199254740991);
    let boundary = policy::timeout_bounds::decimal_timeout(9007199254740992);
    let next = policy::timeout_bounds::decimal_timeout(9007199254740993);
    let last = policy::timeout_bounds::decimal_timeout(u64::MAX);
    assert(!before && boundary && next && last);
    let missing = policy::observed_timeouts::exact_observation_timeout(None);
    let safe = policy::observed_timeouts::exact_observation_timeout(Some(9007199254740991));
    let maximum = policy::observed_timeouts::exact_observation_timeout(Some(u64::MAX));
    assert(missing.is_none() && safe.is_none() && maximum == Some(u64::MAX));
}


fn expect(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn limits_of_executable_operations() {
    expect(!policy::valid_execution_bounds(0, 1));
    expect(!policy::valid_execution_bounds(1, 0));
    expect(policy::valid_execution_bounds(1, 1));
    expect(policy::valid_execution_bounds(u64::MAX, 64 * 1024 * 1024));
    expect(!policy::valid_execution_bounds(1, 64 * 1024 * 1024 + 1));
}

}

#[cfg(not(verus_keep_ghost))]
#[test]
fn exact_plan_bounds_survive_jcs_and_retained_plan_readers() {
    use chrono_judge_routes::{Execution, plan_query, prepare_collection};
    use serde_json::json;
    use std::collections::BTreeMap;
    let method = chrono_judge_routes::Method {
        owner: "checker".into(),
        operation: "run".into(),
        tool: "unused".into(),
        argv: Vec::new(),
        output_contract: None,
    };
    let methods = BTreeMap::from([("run".to_owned(), vec![method])]);
    let actions = BTreeMap::from([("unit".to_owned(), "run".to_owned())]);
    let mut identities = std::collections::BTreeSet::new();
    for n in [
        1,
        (1_u64 << 53) - 1,
        1_u64 << 53,
        (1_u64 << 53) + 1,
        u64::MAX - 1,
        u64::MAX,
    ] {
        let plans = serde_json::from_value(json!({"unit":{"operations":["run"], "timeout_seconds":n,
            "output_limit_bytes":8, "operation_bounds":{"run":{"timeout_seconds":n, "output_limit_bytes":8}}}})).unwrap();
        let plan = prepare_collection(
            std::path::Path::new("/retained"),
            json!({"run":"~i:9007199254740993"}),
            &actions.keys().cloned().collect(),
            &plans,
            &methods,
            &actions,
            BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(plan.schema, "chrono-execution-plan/v2");
        assert!(identities.insert(plan.identity.clone()));
        let bytes = serde_json::to_vec(&plan).unwrap();
        let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let timeout = &wire["operations"][0]["timeout_seconds"];
        if n < 1_u64 << 53 {
            assert_eq!(timeout.as_u64(), Some(n));
        } else {
            assert_eq!(timeout.as_str().unwrap(), n.to_string());
        }
        assert_eq!(
            serde_json::from_slice::<Execution>(&bytes)
                .unwrap()
                .operations[0]
                .timeout_seconds,
            n
        );
        assert_eq!(
            plan_query::check_plan(&*bytes).unwrap()["plan"],
            plan.identity
        );
        assert_eq!(
            plan_query::dispatch_plan(&*bytes).unwrap()["plan"],
            plan.identity
        );
        let mut old = wire;
        old["schema"] = json!("chrono-execution-plan/v1");
        old["binding"] = json!({});
        old.as_object_mut().unwrap().remove("identity");
        for bound in [
            "/operations/0/timeout_seconds",
            "/selected/unit/timeout_seconds",
            "/selected/unit/operation_bounds/run/timeout_seconds",
        ] {
            *old.pointer_mut(bound).unwrap() = json!(n);
        }
        // The v1 fixture uses ordinary integer tokens. All safe inputs have the
        // old JCS spelling; wider historical identities must be regenerated.
        old["identity"] = json!(chrono_judge_routes::sha256(old.to_string().as_bytes()));
        let legacy: Execution = serde_json::from_value(old).unwrap();
        assert_eq!(legacy.validate().is_ok(), n < 1_u64 << 53);
        assert_eq!(legacy.with_scheduling(None).is_ok(), n < 1_u64 << 53);
    }
}
