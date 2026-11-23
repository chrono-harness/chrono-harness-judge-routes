//! Read and replay route evidence without executing operations or acquiring tools.
use crate::declarations as execution;
use crate::{Execution, Method, Receipt, compare_validated, unique_actions};
use chrono_harness::wire::Request;
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

/// Check the receipt for one operation taken from the caller's already admitted plan.
pub fn check_operation_receipt(input: impl Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        plan: String,
        root: PathBuf,
        environment: BTreeMap<String, String>,
        operation: crate::Operation,
        tool: Option<chrono_harness::observation::Tool>,
        receipt: Option<Receipt>,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let problem = crate::compare_operation_receipt(
        &q.plan,
        &q.root,
        &q.environment,
        &q.operation,
        q.tool.as_ref(),
        q.receipt.as_ref(),
    )
    .err();
    Ok(json!({"plan":q.plan, "operation":q.operation.method.operation, "error":problem}))
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

#[derive(Deserialize)]
struct Declarations {
    plans: BTreeMap<String, execution::Plan>,
    methods: Result<BTreeMap<String, Vec<Method>>, String>,
}

fn declarations(
    filemap: Value,
    projects: Value,
    read: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<Declarations, String> {
    let response = read(
        "execution-declarations-v1",
        &json!({"filemap":filemap,"projects":projects}),
    )?;
    serde_json::from_slice(&response)
        .map_err(|e| format!("invalid registration execution declarations: {e}"))
}

fn installed(operation: &str, input: &Value) -> Result<Vec<u8>, String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    crate::registration::query(&root, operation, input)
}

/// Reconstruct the global declaration plan at its original coordinates.
pub fn collection_plan(input: impl Read) -> Result<Value, String> {
    collection_plan_with(input, installed)
}

pub fn collection_plan_with(
    input: impl Read,
    read: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<Value, String> {
    let q: Collection = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let declared = declarations(q.filemap, q.projects, read)?;
    let plan = crate::prepare_collection(
        &q.root,
        q.binding,
        &q.selected,
        &declared.plans,
        &declared.methods?,
        &unique_actions(q.test_operations),
        q.environment,
    )?;
    serde_json::to_value(plan).map_err(|e| e.to_string())
}

/// Plan the full CI obligations from the interpreted endpoint, without running tools.
pub fn full_operations(input: impl Read) -> Result<Value, String> {
    full_operations_with(input, installed)
}

pub fn full_operations_with(
    input: impl Read,
    read: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        selected: BTreeSet<String>,
        filemap: Value,
        projects: Value,
        test_operations: BTreeMap<String, Vec<String>>,
        execution_units: Option<Value>,
    }
    let query: Query = serde_json::from_reader(input).map_err(|error| error.to_string())?;
    let declared = declarations(query.filemap, query.projects, read)?;
    let plans = declared.plans;
    let required_units = query
        .execution_units
        .as_ref()
        .map(|block| {
            let units = crate::validate_unit_assignments(block, &plans)?;
            Ok::<_, String>(chrono_harness::units::required(&units, &query.selected))
        })
        .transpose()?;
    let methods = declared.methods?;
    let (selected, operations) = crate::order(
        &query.selected,
        &plans,
        &methods,
        &unique_actions(query.test_operations),
    )?;
    Ok(
        json!({"selected": selected, "operations": operations, "methods": methods,
        "required_units": required_units}),
    )
}

/// Rebuild the global operation graph and a unit's expanded operations at its original coordinates.
pub fn unit_declarations(input: impl Read) -> Result<Value, String> {
    unit_declarations_with(input, installed)
}

pub fn unit_declarations_with(
    input: impl Read,
    read: impl FnOnce(&str, &Value) -> Result<Vec<u8>, String>,
) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        request: Request,
        effective_inputs: Value,
        filemap: Value,
        projects: Value,
        test_operations: BTreeMap<String, Vec<String>>,
        global: BTreeSet<String>,
        unit: BTreeSet<String>,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    if !q.unit.is_subset(&q.global) {
        return Err("E_UNIT_ASSIGNMENT: unit selection exceeds global selection".into());
    }
    let binding = crate::binding(&q.request, &q.effective_inputs);
    let declared = declarations(q.filemap, q.projects, read)?;
    let plans = declared.plans;
    let methods = declared.methods?;
    let actions = unique_actions(q.test_operations);
    let (chosen, ordered) = crate::order(&q.global, &plans, &methods, &actions)?;
    let (selected, mut operations) = crate::order(&q.unit, &plans, &methods, &actions)?;
    for operation in &mut operations {
        operation.method.argv = crate::expand(&operation.method.argv, &binding);
    }
    Ok(json!({
        "request_id":q.request.request_id, "binding":binding,
        "global_graph":{"schema":"chrono-global-operation-graph/v1","selected":chosen,"operations":ordered},
        "selected":selected,"operations":operations,
    }))
}

/// Check a retained tool against its original declaration without observing a live tool.
pub fn retained_tool(input: impl Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        tool: chrono_harness::observation::Tool,
        program: String,
        version_argv: Vec<String>,
        environment: BTreeMap<String, String>,
        root: PathBuf,
        expected_version: Option<String>,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    crate::validate_retained_tool(
        &q.tool,
        &q.program,
        &q.version_argv,
        &q.environment,
        &q.root,
        q.expected_version.as_deref(),
    )?;
    Ok(json!({"sha256":q.tool.sha256}))
}

pub fn check_plan(input: impl Read) -> Result<Value, String> {
    let plan: Execution = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    plan.validate()?;
    Ok(json!({"plan":plan.identity}))
}

/// Admit a plan and resolve the order in which its executor considers ready operations.
pub fn dispatch_plan(input: impl Read) -> Result<Value, String> {
    let plan: Execution = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    plan.validate_graph()?;
    let dispatch = execution::Dispatch::for_plan(&plan)?;
    Ok(json!({"plan": plan.identity, "dispatch": dispatch}))
}

/// Expand the caller's scoped request fields using the same original-directory
/// convention as the full planner.
fn scoped_binding(mut binding: Value, report_path: &str) -> Result<Value, String> {
    chrono_harness::relative_path(report_path)?;
    let fields = binding
        .as_object_mut()
        .ok_or("E_PLAN_BINDING: expected scoped request fields")?;
    if fields.contains_key("operation_state") {
        return Err("E_PLAN_BINDING: operation_state is derived from the report".into());
    }
    let run = fields
        .get("run")
        .filter(|run| run.is_string())
        .ok_or("E_PLAN_BINDING: missing scoped run identity")?;
    let directory = crate::operation_state_path(report_path, run);
    fields.insert("operation_state".into(), Value::String(directory));
    Ok(binding)
}

/// Replay the original report selection without acquiring tools or running operations.
pub fn check_scoped_plan(input: impl Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        plan: Execution,
        binding: Value,
        report_path: String,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    q.plan.validate()?;
    if q.plan.binding != scoped_binding(q.binding, &q.report_path)? {
        return Err(
            "E_PLAN_BINDING: scoped plan differs from the original request and report".into(),
        );
    }
    Ok(json!({"plan":q.plan.identity}))
}

/// Scoped CI can order the same declarations for scheduling and replay them
/// for a particular request without probing an executable during collection.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopedGraph {
    selected: BTreeSet<String>,
    plans: BTreeMap<String, execution::Plan>,
    methods: BTreeMap<String, Vec<Method>>,
    execute: BTreeMap<String, String>,
}

pub fn scoped_operations(input: impl Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        graph: ScopedGraph,
        binding: Option<Value>,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let graph = q.graph;
    let (selected, mut operations) = crate::order(
        &graph.selected,
        &graph.plans,
        &graph.methods,
        &graph.execute,
    )?;
    if let Some(binding) = q.binding {
        for op in &mut operations {
            op.method.argv = crate::expand(&op.method.argv, &binding);
        }
    }
    Ok(json!({"selected": selected, "operations": operations}))
}

pub fn prepare_scoped_plan(root: &std::path::Path, input: impl Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        graph: ScopedGraph,
        binding: Value,
        report_path: Option<String>,
        declarations: Value,
        environment: BTreeMap<String, String>,
        reused: BTreeMap<String, chrono_harness::observation::Tool>,
        scheduling: Option<execution::Scheduling>,
    }
    let q: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let binding = match q.report_path {
        Some(path) => scoped_binding(q.binding, &path)?,
        None => q.binding,
    };
    let graph = q.graph;
    let plan = crate::prepare_scheduled(
        root,
        binding,
        &graph.selected,
        &graph.plans,
        &graph.methods,
        &graph.execute,
        &q.declarations,
        q.environment,
        &q.reused,
        q.scheduling,
        false,
    )?;
    serde_json::to_value(plan).map_err(|e| e.to_string())
}
