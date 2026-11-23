//! Read and replay route evidence without executing operations or acquiring tools.
use crate::{Execution, Receipt, compare_validated, unique_actions};
use chrono_harness::wire::Request;
use chrono_judge_registration::execution;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
};

/// Compare the supplied original receipts against a single validated plan, in plan order.
pub fn check_plan_receipts(input: impl std::io::Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        plan: Execution,
        receipts: BTreeMap<String, Receipt>,
    }
    let Query { plan, receipts } =
        serde_json::from_reader(input).map_err(|error| error.to_string())?;
    plan.validate()?;
    let mut errors = BTreeMap::new();
    for operation in &plan.operations {
        let name = &operation.method.operation;
        errors.insert(
            name,
            compare_validated(&plan, operation, receipts.get(name)).err(),
        );
    }
    Ok(json!({"plan":plan.identity,"operations":errors}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    request: Request,
    effective_inputs: Value,
    templates: BTreeMap<String, Vec<String>>,
}

/// Compute the request binding and expand only the original declaration templates.
pub fn plan_context(input: impl Read) -> Result<Value, String> {
    let query: Context = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let binding = crate::binding(&query.request, &query.effective_inputs);
    let expanded: BTreeMap<_, _> = query
        .templates
        .into_iter()
        .map(|(id, argv)| (id, crate::expand(&argv, &binding)))
        .collect();
    Ok(json!({"request_id":query.request.request_id,"binding":binding,"expanded":expanded}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    root: PathBuf,
    binding: Value,
    selected: BTreeSet<String>,
    filemap: Value,
    projects: Value,
    test_operations: BTreeMap<String, Vec<String>>,
    environment: BTreeMap<String, String>,
}

/// Reconstruct the global declaration plan at its original coordinates.
pub fn collection_plan(input: impl Read) -> Result<Value, String> {
    let q: Collection = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let plan = crate::prepare_collection(
        &q.root,
        q.binding,
        &q.selected,
        &execution::plans(&q.filemap)?,
        &execution::methods(&q.projects)?,
        &unique_actions(q.test_operations),
        q.environment,
    )?;
    serde_json::to_value(plan).map_err(|e| e.to_string())
}
