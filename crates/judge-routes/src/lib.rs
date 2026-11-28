#![allow(unexpected_cfgs)]
// Routes owns method planning and comparison with runner-produced observations.
#[cfg(not(verus_keep_ghost))]
mod commands;
#[cfg(not(verus_keep_ghost))]
pub use commands::runner_program;
#[cfg(not(verus_keep_ghost))]
pub mod evidence;
#[cfg(not(verus_keep_ghost))]
mod units;
#[cfg(not(verus_keep_ghost))]
pub use evidence::{CompiledArtifact, OutputContract, ProcessResult, Tool};
#[cfg(not(verus_keep_ghost))]
pub mod protocol;
#[cfg(not(verus_keep_ghost))]
use protocol::{Request, Scope};

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// ASCII encoding of one hexadecimal digit of a digest.
fn hex_digit(n: u8) -> (digit: u8)
    requires n < 16,
    ensures digit as int == if n < 10 { 48 + n as int } else { 87 + n as int },
{
    if n < 10 { 48 + n } else { 87 + n }
}

pub(crate) fn hex_pair(byte: u8) -> (digits: (u8, u8))
    ensures
        digits.0 as int == if byte / 16 < 10 { 48 + byte as int / 16 } else { 87 + byte as int / 16 },
        digits.1 as int == if byte % 16 < 10 { 48 + byte as int % 16 } else { 87 + byte as int % 16 },
{
    (hex_digit(byte / 16), hex_digit(byte % 16))
}

/// Select the coordinates of the original request without interpolating literals.
pub fn endpoint_argument<'a>(literal: &'a str, base: &'a str, candidate: &'a str)
    -> (expanded: &'a str)
    ensures expanded@ == if literal@ == "{base}"@ { base@ }
        else if literal@ == "{candidate}"@ { candidate@ } else { literal@ },
{
    if <&str>::exec_eq(literal, "{base}") { base }
    else if <&str>::exec_eq(literal, "{candidate}") { candidate }
    else { literal }
}

/// The runner's compiler protocol places its output argument at slot ten.
pub fn compiler_output(argv: &[String]) -> (output: Option<&String>)
    ensures match output {
        Some(arg) => argv@.len() > 10 && *arg == argv@[10],
        None => argv@.len() <= 10,
    },
{
    if argv.len() > 10 { Some(&argv[10]) } else { None }
}

pub fn compiler_directory<T>(directory: Option<T>, output_template: bool) -> (result: Option<T>)
    ensures result == if output_template { directory } else { None },
{
    if output_template { directory } else { None }
}

/// JSON and canonical hashing supply the ordered DELTA comparison and plan digest.
pub fn admit_impact(schema: &str, same_delta: bool) -> (result: Result<(), &'static str>)
    ensures
        (result is Ok) == (schema@ == "chrono-filemap-impact/v6"@ && same_delta),
        (result matches Err(e) ==> e@ == "E_IMPACT: identity"@),
{
    if <&str>::exec_eq(schema, "chrono-filemap-impact/v6") && same_delta {
        Ok(())
    } else {
        Err("E_IMPACT: identity")
    }
}

pub fn admit_plan_schema<F>(schema: &str, legacy_has_large_timeout: F) -> (result: Result<(), &'static str>)
where
    F: FnOnce() -> bool,
    requires legacy_has_large_timeout.requires(()),
    ensures
        schema@ == "chrono-execution-plan/v2"@ ==> result is Ok,
        schema@ != "chrono-execution-plan/v2"@ && schema@ != "chrono-execution-plan/v1"@
            ==> (result matches Err(e) && e@ == "E_PLAN_IDENTITY: unknown plan schema"@),
        schema@ == "chrono-execution-plan/v1"@ ==>
            exists|large: bool| #[trigger] legacy_has_large_timeout.ensures((), large) &&
                if large {
                    result matches Err(e) &&
                        e@ == "E_PLAN_IDENTITY: regenerate legacy plan from its exact timeout declarations"@
                } else { result is Ok },
{
    proof {
        reveal_strlit("chrono-execution-plan/v1");
        reveal_strlit("chrono-execution-plan/v2");
        assert("chrono-execution-plan/v1"@[23] != "chrono-execution-plan/v2"@[23]);
    }
    if <&str>::exec_eq(schema, "chrono-execution-plan/v2") { return Ok(()); }
    if !<&str>::exec_eq(schema, "chrono-execution-plan/v1") {
        return Err("E_PLAN_IDENTITY: unknown plan schema");
    }
    if legacy_has_large_timeout() {
        Err("E_PLAN_IDENTITY: regenerate legacy plan from its exact timeout declarations")
    } else {
        Ok(())
    }
}
}

#[cfg(not(verus_keep_ghost))]
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let (high, low) = hex_pair(byte);
        encoded.push(char::from(high));
        encoded.push(char::from(low));
    }
    encoded
}
/// Semantic JSON identities use RFC 8785 binary64 numbers. Execution bounds
/// provide their exact typed wire representation before this shared hash.
#[cfg(not(verus_keep_ghost))]
pub fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    digest_value(value)
}

#[cfg(not(verus_keep_ghost))]
fn digest_value(mut value: Value) -> Result<String, String> {
    fn binary64(value: &mut Value) {
        match value {
            Value::Array(values) => values.iter_mut().for_each(binary64),
            Value::Object(fields) => fields.values_mut().for_each(binary64),
            Value::Number(n)
                if n.as_u64().is_some_and(|n| n > serde_jcs::MAX_SAFE_INTEGER)
                    || n.as_i64()
                        .is_some_and(|n| n < -(serde_jcs::MAX_SAFE_INTEGER as i64)) =>
            {
                *value = Value::from(n.as_f64().expect("finite JSON number"));
            }
            _ => {}
        }
    }
    binary64(&mut value);
    Ok(sha256(
        &serde_jcs::to_vec(&value).map_err(|e| e.to_string())?,
    ))
}

#[cfg(not(verus_keep_ghost))]
mod declarations;
#[cfg(not(verus_keep_ghost))]
pub mod verified_admission;
#[cfg(not(verus_keep_ghost))]
pub mod verified_bounds;
#[cfg(not(verus_keep_ghost))]
pub use declarations::{Plan, Scheduling};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// The declared method carried by each operation in the public route plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<OutputContract>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Operation {
    pub method: Method,
    pub predecessors: BTreeSet<String>,
    #[serde(with = "crate::declarations::timeout_wire")]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Execution {
    pub schema: String,
    pub identity: String,
    pub binding: Value,
    pub root: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
    pub tools: BTreeMap<String, Tool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<Scheduling>,
}
#[cfg(not(verus_keep_ghost))]
impl Execution {
    fn digest(&self) -> Result<String, String> {
        let mut value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        value.as_object_mut().unwrap().remove("identity");
        admit_plan_schema(&self.schema, || {
            use declarations::decimal_timeout;
            self.operations
                .iter()
                .any(|op| decimal_timeout(op.timeout_seconds))
                || self.selected.values().any(|plan| {
                    decimal_timeout(plan.timeout_seconds)
                        || plan
                            .operation_bounds
                            .values()
                            .any(|bounds| decimal_timeout(bounds.timeout_seconds))
                })
        })?;
        digest_value(value)
    }
    fn validate_graph(&self) -> Result<(), String> {
        if !verified_admission::plan_identity_matches(&self.identity, &self.digest()?) {
            return Err("E_PLAN_IDENTITY".into());
        }
        let mut preceding = BTreeSet::new();
        for op in &self.operations {
            if !verified_admission::predecessors_ready(&op.predecessors, &preceding)
                || !verified_admission::insert_once(&mut preceding, op.method.operation.clone())
                || !verified_bounds::valid_execution_bounds(
                    op.timeout_seconds,
                    op.output_limit_bytes,
                )
            {
                return Err(
                    "E_EXECUTION_PLAN: duplicate, unordered prerequisite or invalid bound".into(),
                );
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_graph()?;
        if let Some(policy) = &self.scheduling {
            policy.indices(&self.operations)?;
        }
        Ok(())
    }
    pub fn with_scheduling(mut self, policy: Option<Scheduling>) -> Result<Self, String> {
        self.validate()?;
        self.schema = "chrono-execution-plan/v2".into();
        self.scheduling = policy;
        self.identity = self.digest()?;
        self.validate()?;
        Ok(self)
    }
}
#[cfg(not(verus_keep_ghost))]
pub fn binding(req: &Request, inputs: &Value) -> Value {
    json!({"base":req.base.commit,"base_tree":req.base.tree,"candidate":req.candidate.commit,"candidate_tree":req.candidate.tree,"registry":req.registries.digest,"context":req.context.sha256,"inputs":inputs,"entry":req.observations["entry"],"run":req.observations["run"],"operation_state":req.observations["report_path"].as_str().map(|path| operation_state_path(path, &req.observations["run"]))})
}
/// Keep operation originals beside the report selected by the existing owner.
#[cfg(not(verus_keep_ghost))]
pub fn operation_state_path(report_path: &str, run: &Value) -> String {
    plan_query::operation_directory(
        report_path,
        &digest(run).expect("JSON run identity serializes"),
    )
}
#[cfg(not(verus_keep_ghost))]
pub fn expand(argv: &[String], binding: &Value) -> Result<Vec<String>, String> {
    let base = binding["base"].as_str().unwrap_or("");
    let candidate = binding["candidate"].as_str().unwrap_or("");
    let args: Vec<String> = argv
        .iter()
        .map(|arg| endpoint_argument(arg, base, candidate).to_owned())
        .collect();
    let directory = compiler_directory(
        binding["operation_state"].as_str(),
        compiler_output(&args).is_some_and(|arg| arg.starts_with("{operation_state}/")),
    );
    if let Some(directory) = directory {
        return serde_json::from_value(evidence::compiler(
            json!({"operation":"bind","directory":directory,"argv":args}),
        )?)
        .map_err(|e| e.to_string());
    }
    Ok(args)
}
/// Merge explicit sequences into a DAG before observing tools or launching any command.
#[cfg(not(verus_keep_ghost))]
pub fn order(
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
) -> Result<(BTreeMap<String, Plan>, Vec<Operation>), String> {
    let mut operations = verified_admission::OperationGraph::new();
    let mut chosen = BTreeMap::new();
    for test in selected {
        let plan = verified_admission::required_declaration(plans.get(test), ())
            .map_err(|()| format!("E_ROUTE_MISSING: execution plan {test}"))?;
        let steps = plan.steps().map_err(|e| format!("{e}: {test}"))?;
        let action = verified_admission::required_declaration(execute.get(test), ())
            .map_err(|()| format!("E_ROUTE_MISSING: unique execute action for {test}"))?;
        if !verified_admission::contains_action(&plan.operations, action) {
            return Err(format!("E_EXECUTION_PLAN: {test} omits its execute action"));
        }
        let mut previous = None;
        for (id, bounds) in steps {
            let method = verified_admission::admit_method(id.as_str(), methods.get(id)).map_err(
                |issue| match issue {
                    verified_admission::MethodIssue::Missing => format!("E_ROUTE_MISSING: {id}"),
                    verified_admission::MethodIssue::Ambiguous => {
                        format!("E_ROUTE_AMBIGUOUS: {id}")
                    }
                },
            )?;
            if !verified_admission::merge_operation(
                &mut operations,
                id.to_owned(),
                (method, bounds.timeout_seconds, bounds.output_limit_bytes),
                previous,
            ) {
                return Err(format!("E_PLAN_BOUNDS: inconsistent shared operation {id}"));
            }
            previous = Some(id.to_owned());
        }
        chosen.insert(test.clone(), plan.clone());
    }
    let ordered = verified_admission::order_operations(operations)?
        .into_iter()
        .map(
            |(_, ((method, timeout_seconds, output_limit_bytes), predecessors))| Operation {
                method: method.clone(),
                predecessors,
                timeout_seconds,
                output_limit_bytes,
            },
        )
        .collect();
    Ok((chosen, ordered))
}
#[cfg(not(verus_keep_ghost))]
pub fn prepare(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
) -> Result<Execution, String> {
    prepare_inner(
        root,
        binding,
        selected,
        plans,
        methods,
        execute,
        declarations,
        environment,
        reused,
        true,
        None,
    )
}
/// Registered full/scoped callers share one preparation owner and process engine.
#[cfg(not(verus_keep_ghost))]
pub fn prepare_scheduled(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
    scheduling: Option<Scheduling>,
    strict: bool,
) -> Result<Execution, String> {
    prepare_inner(
        root,
        binding,
        selected,
        plans,
        methods,
        execute,
        declarations,
        environment,
        reused,
        strict,
        scheduling,
    )
}
#[cfg(not(verus_keep_ghost))]
fn prepare_inner(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    mut environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
    strict: bool,
    scheduling: Option<Scheduling>,
) -> Result<Execution, String> {
    #[cfg(unix)]
    environment.remove("CHRONO_PROCESS_FDS");
    let (selected, mut operations) = order(selected, plans, methods, execute)?;
    if let Some(policy) = &scheduling {
        policy.indices(&operations)?;
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut tools = BTreeMap::new();
    for op in &mut operations {
        op.method.argv = expand(&op.method.argv, &binding)?;
        let id = &op.method.tool;
        if tools.contains_key(id) {
            continue;
        }
        let defs: Vec<_> = declarations
            .as_array()
            .ok_or("E_ROUTE_MISSING: tool declarations")?
            .iter()
            .filter(|t| t["id"] == *id)
            .collect();
        let Some(&t) = verified_admission::unique_action(&defs) else {
            return Err(format!("E_ROUTE_AMBIGUOUS: tool {id}"));
        };
        let program = t["program"].as_str().ok_or("tool program")?;
        let argv: Vec<String> =
            serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
        let tool = verified_admission::retain_or_observe(reused.get(id).cloned(), || {
            evidence::tool(
                &root,
                program,
                &argv,
                &environment,
                op.timeout_seconds,
                op.output_limit_bytes,
            )
        })?;
        if strict || t["expected_version"].is_string() {
            validate_tool(
                &tool,
                program,
                &argv,
                &environment,
                &root,
                t["expected_version"].as_str(),
            )
            .map_err(|cause| {
                format!(
                    "{cause}: {} operation {} (tool {id})",
                    op.method.owner, op.method.operation
                )
            })?;
        }
        tools.insert(id.clone(), tool);
    }
    let mut plan = Execution {
        schema: "chrono-execution-plan/v2".into(),
        identity: String::new(),
        binding,
        root,
        environment,
        selected,
        operations,
        tools,
        scheduling,
    };
    plan.identity = plan.digest()?;
    Ok(plan)
}
#[cfg(not(verus_keep_ghost))]
pub use verified_admission::process_success;

/// Construct the current global declaration graph without observing any business executable.
#[cfg(not(verus_keep_ghost))]
pub fn prepare_collection(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    environment: BTreeMap<String, String>,
) -> Result<Execution, String> {
    let (selected, mut operations) = order(selected, plans, methods, execute)?;
    for op in &mut operations {
        op.method.argv = expand(&op.method.argv, &binding)?;
    }
    let mut plan = Execution {
        schema: "chrono-execution-plan/v2".into(),
        identity: String::new(),
        binding,
        root: root.into(),
        environment,
        selected,
        operations,
        tools: BTreeMap::new(),
        scheduling: None,
    };
    plan.identity = plan.digest()?;
    Ok(plan)
}
#[cfg(not(verus_keep_ghost))]
pub fn validate_retained_tool(
    tool: &Tool,
    program: &str,
    argv: &[String],
    env: &BTreeMap<String, String>,
    root: &Path,
    expected: Option<&str>,
) -> Result<(), String> {
    validate_tool_inner(tool, program, argv, env, root, expected, false)
}
#[cfg(not(verus_keep_ghost))]
pub fn validate_tool(
    tool: &Tool,
    program: &str,
    argv: &[String],
    env: &BTreeMap<String, String>,
    root: &Path,
    expected: Option<&str>,
) -> Result<(), String> {
    validate_tool_inner(tool, program, argv, env, root, expected, true)
}
#[cfg(not(verus_keep_ghost))]
fn validate_tool_inner(
    tool: &Tool,
    program: &str,
    argv: &[String],
    env: &BTreeMap<String, String>,
    root: &Path,
    expected: Option<&str>,
    live: bool,
) -> Result<(), String> {
    let actual = &tool.version;
    process_success(actual).map_err(|e| format!("E_TOOL_BINDING: {e}"))?;
    let executable = tool.path.to_str().ok_or("tool path")?;
    let matches = verified_admission::retained_version_matches(
        verified_admission::VersionInvocation {
            absolute_path: tool.path.is_absolute(),
            same_basename: tool.path.file_name().and_then(|s| s.to_str()) == Some(&tool.basename),
            same_executable: actual.executable == tool.path,
            same_argv: actual
                .argv
                .iter()
                .map(String::as_str)
                .eq(std::iter::once(executable).chain(argv.iter().map(String::as_str))),
            same_directory: actual.cwd == root,
            same_environment: actual.environment == *env,
            same_stdout_bytes: actual.stdout.as_bytes() == actual.stdout_bytes,
        },
        &tool.program,
        program,
        &tool.sha256,
        &actual.sha256,
        actual.stdout.trim_end(),
        expected,
    );
    verified_admission::accept_tool_version(matches, live, || {
        fs::read(&tool.path)
            .map(|bytes| crate::sha256(&bytes))
            .is_ok_and(|digest| digest == tool.sha256)
    })?;
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn validate_invocation(
    root: &Path,
    entry: &Value,
    template: &[String],
    binding: &Value,
) -> Result<(), String> {
    let bytes = registration::runner(
        root,
        "check-invocation-v1",
        &json!({"mode":"template","root":root,
        "entry":entry,"argv":template,"binding":binding}),
    )?;
    serde_json::from_slice::<()>(&bytes).map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn canonical(req: &Request, config: &Value) -> Result<(), String> {
    let bytes = registration::runner(
        &req.candidate.root,
        "check-invocation-v1",
        &json!({"mode":"request","request":req,"config":config}),
    )?;
    serde_json::from_slice::<()>(&bytes).map_err(|e| e.to_string())
}

/// Validate the complete registered unit assignment before filtering a unit.
/// The full route owns this check so projects receives an already ordered,
/// contribution-sized plan and cannot make a selector hide a global cycle or
/// replacement obligation.
#[cfg(not(verus_keep_ghost))]
fn unit_selection(
    req: &Request,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
) -> Result<BTreeSet<String>, String> {
    let Some(scope) = &req.scope else {
        return Ok(selected.clone());
    };
    let definitions = validate_unit_assignments(&req.observations["execution_units"], plans)?;
    match scope {
        Scope::Unit { unit } => crate::units::select(&definitions, selected, unit)
            .map_err(|e| format!("E_UNIT_ASSIGNMENT: {e}")),
        Scope::Collect { .. } => Ok(selected.clone()),
    }
}

/// Validate the explicit assignment independently of invocation or SDK acquisition.
#[cfg(not(verus_keep_ghost))]
pub fn validate_unit_assignments(
    block: &Value,
    plans: &BTreeMap<String, Plan>,
) -> Result<BTreeMap<String, crate::units::Unit>, String> {
    let units = block["units"]
        .as_object()
        .ok_or("E_UNIT_ASSIGNMENT: missing execution_units")?;
    let mut definitions = BTreeMap::new();
    for (name, value) in units {
        let unit: crate::units::Unit =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        definitions.insert(name.clone(), unit);
    }
    let mut operation_sets = BTreeMap::<String, BTreeSet<String>>::new();
    for (name, unit) in &definitions {
        for test in &unit.tests {
            let plan = plans
                .get(test)
                .ok_or_else(|| format!("E_UNIT_ASSIGNMENT: unknown plan {test}"))?;
            for operation in &plan.operations {
                operation_sets
                    .entry(operation.clone())
                    .or_default()
                    .insert(name.clone());
            }
        }
    }
    let shared: BTreeMap<String, Vec<String>> =
        serde_json::from_value(block["shared_operations"].clone()).map_err(|e| e.to_string())?;
    let plan_ids = plans.keys().cloned().collect();
    crate::units::assignments(&definitions, &plan_ids, &shared, &operation_sets)
        .map_err(|e| format!("E_UNIT_ASSIGNMENT: {e}"))?;
    Ok(definitions)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Receipt {
    pub plan: String,
    pub operation: String,
    pub method: String,
    #[serde(serialize_with = "crate::declarations::timeout_wire::observation")]
    pub process: ProcessResult,
    pub compiler_dependency: Option<Result<Vec<u8>, String>>,
    pub compiler_artifact: Option<Result<CompiledArtifact, String>>,
    pub digest: String,
}

#[cfg(not(verus_keep_ghost))]
impl Receipt {
    fn identity(&self) -> Result<String, String> {
        let mut v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        v.as_object_mut().unwrap().remove("digest");
        digest_value(v)
    }
}
/// Compare actual producer observations, never populate missing observations from a plan.
#[cfg(not(verus_keep_ghost))]
pub fn compare(plan: &Execution, op: &Operation, receipt: Option<&Receipt>) -> Result<(), String> {
    plan.validate()?;
    compare_validated(plan, op, receipt)
}

#[cfg(not(verus_keep_ghost))]
fn compare_validated(
    plan: &Execution,
    op: &Operation,
    receipt: Option<&Receipt>,
) -> Result<(), String> {
    compare_operation_receipt(
        &plan.identity,
        &plan.root,
        &plan.environment,
        op,
        plan.tools.get(&op.method.tool),
        receipt,
    )
}

#[cfg(not(verus_keep_ghost))]
fn compare_operation_receipt(
    plan_identity: &String,
    root: &Path,
    environment: &BTreeMap<String, String>,
    op: &Operation,
    tool: Option<&Tool>,
    receipt: Option<&Receipt>,
) -> Result<(), String> {
    let problem = |code: &str, message: &str| {
        format!(
            "{code}: {} operation {} ({}): {message}",
            op.method.owner, op.method.operation, op.method.tool
        )
    };
    let r = receipt.ok_or_else(|| problem("E_ROUTE_MISSING", "missing execution receipt"))?;
    let tool = tool.ok_or_else(|| problem("E_TOOL_BINDING", "missing tool"))?;
    let p = &r.process;
    let mut argv = vec![tool.path.to_str().ok_or("tool path")?.into()];
    argv.extend(op.method.argv.clone());
    if p.argv != argv {
        return Err(problem("E_RECEIPT_MISMATCH", "invocation arguments differ"));
    }
    if !verified_admission::receipt_target_matches(
        &r.plan,
        plan_identity,
        &r.operation,
        &op.method.operation,
    ) || !verified_admission::receipt_seal_matches(
        &r.method,
        digest(&op.method),
        &r.digest,
        || r.identity(),
    )? || !verified_admission::receipt_tool_matches(
        p.cwd == root && p.executable == tool.path,
        &p.sha256,
        &tool.sha256,
    ) || !verified_admission::receipt_input_matches(
        &p.environment == environment,
        &p.environment_digest,
        || digest(environment),
        &p.stdin_sha256,
        || crate::sha256(&[]),
    )? || !verified_admission::output_sizes_fit(
        p.stdout_bytes.len(),
        p.stderr_bytes.len(),
        op.output_limit_bytes,
    ) || !verified_admission::stream_matches(
        &p.stdout_sha256,
        &crate::sha256(&p.stdout_bytes),
        p.stdout == String::from_utf8_lossy(&p.stdout_bytes),
    ) || !verified_admission::stream_matches(
        &p.stderr_sha256,
        &crate::sha256(&p.stderr_bytes),
        p.stderr == String::from_utf8_lossy(&p.stderr_bytes),
    ) {
        return Err(problem(
            "E_RECEIPT_MISMATCH",
            "plan/method/tool/cwd/input/environment/output",
        ));
    }
    if !verified_admission::compiler_check_required(
        op.method.output_contract.is_some(),
        r.compiler_dependency.is_some(),
        r.compiler_artifact.is_some(),
    ) {
        return Ok(());
    }
    evidence::compiler(json!({"operation":"check","contract":op.method.output_contract,"stdout":p.stdout_bytes,
        "argv":op.method.argv,"exit":p.exit_code,"process_failed":p.failure.is_some(),
        "dependency":r.compiler_dependency,"limit":op.output_limit_bytes,"artifact":r.compiler_artifact})).map(|_| ())
}
#[cfg(not(verus_keep_ghost))]
fn unique_actions(
    bindings: impl IntoIterator<Item = (String, Vec<String>)>,
) -> BTreeMap<String, String> {
    bindings
        .into_iter()
        .filter_map(|(id, operations)| {
            verified_admission::unique_action(&operations).map(|operation| (id, operation.clone()))
        })
        .collect()
}
#[cfg(not(verus_keep_ghost))]
pub fn judge(input: impl std::io::Read) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        request: Request,
    }
    let envelope: Envelope = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let mut git = Value::Null;
    let mut outputs = BTreeMap::new();
    let (status, findings) = match evaluate(&envelope.request, &mut git) {
        Ok(result) => {
            outputs = result;
            ("pass", vec![])
        }
        Err(message) => (
            "fail",
            vec![
                json!({"code":message.split(':').next().unwrap_or("E_ROUTES"),
            "level":"error","message":message,"delta_refs":["/request"],"causes":[]}),
            ],
        ),
    };
    if !git.is_null() {
        outputs.insert("git_facts".into(), git);
    }
    Ok(json!({"status":status,"findings":findings,"outputs":outputs}))
}

#[cfg(not(verus_keep_ghost))]
mod impact;
#[cfg(not(verus_keep_ghost))]
pub mod plan_query;
#[cfg(not(verus_keep_ghost))]
mod registration;
#[cfg(not(verus_keep_ghost))]
pub mod selection;
#[cfg(not(verus_keep_ghost))]
pub mod source_plan;

#[cfg(not(verus_keep_ghost))]
fn artifact_directories(config: &Value) -> Result<Vec<&str>, String> {
    config["artifacts"]
        .as_array()
        .ok_or("missing artifact declarations")?
        .iter()
        .map(|row| {
            row["path"]
                .as_str()
                .ok_or_else(|| "invalid artifact path".to_owned())
        })
        .collect()
}

#[cfg(not(verus_keep_ghost))]
fn evaluate(req: &Request, git: &mut Value) -> Result<BTreeMap<String, Value>, String> {
    let views = registration::views(req, git)?;
    let old = views.base;
    let r = views.candidate;
    let view = views.registration_view;
    canonical(req, &r.config)?;
    let methods = views.methods?;
    if methods.contains_key("validate.delta") {
        return Err("E_ROUTE_AMBIGUOUS: validate.delta".into());
    }
    let env = serde_json::from_value(req.observations["environment"]["effective"].clone())
        .map_err(|e| e.to_string())?;
    let impact = impact::decode(
        &req.impact,
        &r.config["protocol"],
        &env,
        |command, input| commands::run(&req.candidate.root, command, input),
    )?;
    admit_impact(&impact.schema, impact.delta == req.delta)?;
    let selected = if impact.retired_tests.is_empty() {
        impact.tests.into_iter().collect()
    } else {
        selection::choose(
            &req.judge_id,
            json!({"mode":"view","view":view,"config":req.config_path}),
            None,
            impact.tests,
            impact.retired_tests,
            |op, input| registration::query(&req.candidate.root, op, input),
        )?
    };
    let inputs = registration::effective_inputs(req, &view, &selected)?;
    let reused = views.reused?;
    let global_plans = views.plans?;
    let actions = unique_actions(views.test_operations);
    let (global_chosen, global_operations) = order(&selected, &global_plans, &methods, &actions)?;
    let old_outputs = artifact_directories(&old.config)?
        .into_iter()
        .map(str::to_owned)
        .collect();
    let outputs = artifact_directories(&r.config)?
        .into_iter()
        .map(str::to_owned)
        .collect();
    let previous = source_plan::Endpoint {
        commit: req.base.commit.clone(),
        filemap: old.filemap,
        projects: old.projects,
        artifacts: old_outputs,
    };
    let candidate = source_plan::Endpoint {
        commit: req.candidate.commit.clone(),
        filemap: r.filemap,
        projects: r.projects,
        artifacts: outputs,
    };
    let coverage = source_plan::coverage(
        &req.candidate.root,
        Some(&previous),
        &candidate,
        &impact.reached,
        &methods,
        &global_operations,
        json!({"mode":"request","request":req}),
    )?;
    if !coverage["git_facts"].is_null() {
        registration::append_source_facts(git, coverage["git_facts"].clone())?;
    }
    if let Some(error) = coverage["error"].as_str() {
        return Err(error.to_owned());
    }
    let global_selected = selected.clone();
    let selected = unit_selection(req, &selected, &global_plans)?;
    let scheduling = views.scheduling?;
    let plan = if matches!(req.scope, Some(Scope::Collect { .. })) {
        prepare_collection(
            &req.candidate.root,
            binding(req, &inputs),
            &selected,
            &global_plans,
            &methods,
            &actions,
            env,
        )?
        .with_scheduling(scheduling.clone())?
    } else {
        prepare_scheduled(
            &req.candidate.root,
            binding(req, &inputs),
            &selected,
            &global_plans,
            &methods,
            &actions,
            &r.config["tools"],
            env,
            &reused,
            scheduling,
            true,
        )?
    };
    // Every launched tool is an explicitly retained external input or a registered candidate file.
    for tool in plan.tools.values() {
        let registered = verified_admission::admit_tool_origin(
            || {
                r.config["environment"]["inputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|i| {
                        req.candidate.root.join(i["location"].as_str().unwrap()) == tool.path
                            && i["sha256"] == tool.sha256
                    })
            },
            || {
                tool.path
                    .strip_prefix(&req.candidate.root)
                    .ok()
                    .and_then(|p| p.to_str())
                    .is_some_and(|p| {
                        candidate.filemap["files"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|f| f["path"] == p)
                    })
            },
        );
        if !registered {
            return Err(format!("E_INPUT_UNDECLARED: tool {}", tool.path.display()));
        }
    }
    Ok(BTreeMap::from([
        (
            "execution_plan".into(),
            serde_json::to_value(&plan).unwrap(),
        ),
        (
            "global_graph".into(),
            json!({"schema":"chrono-global-operation-graph/v1","selected":global_chosen,"operations":global_operations}),
        ),
        (
            "global_selected".into(),
            serde_json::to_value(&global_selected).unwrap(),
        ),
        ("tools".into(), serde_json::to_value(&plan.tools).unwrap()),
        ("effective_inputs".into(), inputs),
        ("registration_view".into(), view),
        ("impact".into(), req.impact.clone()),
    ]))
}
