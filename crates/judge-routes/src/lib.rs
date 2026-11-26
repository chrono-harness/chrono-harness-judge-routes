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
#[cfg(not(verus_keep_ghost))]
pub fn digest<T: serde::Serialize>(v: &T) -> Result<String, String> {
    Ok(sha256(&serde_jcs::to_vec(v).map_err(|e| e.to_string())?))
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
        digest(&value)
    }
    fn validate_graph(&self) -> Result<(), String> {
        if self.schema != "chrono-execution-plan/v1" || self.identity != self.digest()? {
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
    format!(
        "{report_path}.operations-{}",
        digest(run).expect("JSON run identity serializes")
    )
}
#[cfg(not(verus_keep_ghost))]
pub fn expand(argv: &[String], binding: &Value) -> Result<Vec<String>, String> {
    let args: Vec<String> = argv
        .iter()
        .map(|a| match a.as_str() {
            "{base}" => binding["base"].as_str().unwrap_or("").into(),
            "{candidate}" => binding["candidate"].as_str().unwrap_or("").into(),
            _ => a.clone(),
        })
        .collect();
    if let Some(directory) = binding["operation_state"].as_str() {
        if args
            .get(10)
            .is_some_and(|arg| arg.starts_with("{operation_state}/"))
        {
            return serde_json::from_value(evidence::compiler(
                json!({"operation":"bind","directory":directory,"argv":args}),
            )?)
            .map_err(|e| e.to_string());
        }
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
    let mut operations: BTreeMap<String, Operation> = BTreeMap::new();
    let mut chosen = BTreeMap::new();
    for test in selected {
        let plan = plans
            .get(test)
            .ok_or_else(|| format!("E_ROUTE_MISSING: execution plan {test}"))?;
        let steps = plan.steps().map_err(|e| format!("{e}: {test}"))?;
        let action = execute
            .get(test)
            .ok_or_else(|| format!("E_ROUTE_MISSING: unique execute action for {test}"))?;
        if !plan.operations.contains(action) {
            return Err(format!("E_EXECUTION_PLAN: {test} omits its execute action"));
        }
        let mut previous = None;
        for (id, bounds) in steps {
            let timeout_seconds = bounds.timeout_seconds;
            let output_limit_bytes = bounds.output_limit_bytes;
            let defs = methods
                .get(id)
                .ok_or_else(|| format!("E_ROUTE_MISSING: {id}"))?;
            if defs.len() != 1 || id == "validate.delta" {
                return Err(format!("E_ROUTE_AMBIGUOUS: {id}"));
            }
            let op = operations
                .entry(id.to_owned())
                .or_insert_with(|| Operation {
                    method: defs[0].clone(),
                    predecessors: BTreeSet::new(),
                    timeout_seconds,
                    output_limit_bytes,
                });
            if op.timeout_seconds != timeout_seconds || op.output_limit_bytes != output_limit_bytes
            {
                return Err(format!("E_PLAN_BOUNDS: inconsistent shared operation {id}"));
            }
            if let Some(p) = previous {
                op.predecessors.insert(p);
            }
            previous = Some(id.to_owned());
        }
        chosen.insert(test.clone(), plan.clone());
    }
    let mut ordered = vec![];
    let mut done = BTreeSet::new();
    while !operations.is_empty() {
        let id = operations
            .iter()
            .find(|(_, op)| verified_admission::predecessors_ready(&op.predecessors, &done))
            .map(|(id, _)| id.clone())
            .ok_or("E_PLAN_CYCLE: inconsistent operation ordering")?;
        ordered.push(operations.remove(&id).unwrap());
        done.insert(id);
    }
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
        if defs.len() != 1 {
            return Err(format!("E_ROUTE_AMBIGUOUS: tool {id}"));
        }
        let t = defs[0];
        let program = t["program"].as_str().ok_or("tool program")?;
        let argv: Vec<String> =
            serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
        let tool = if let Some(tool) = reused.get(id) {
            tool.clone()
        } else {
            evidence::tool(
                &root,
                program,
                &argv,
                &environment,
                op.timeout_seconds,
                op.output_limit_bytes,
            )?
        };
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
        schema: "chrono-execution-plan/v1".into(),
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
        schema: "chrono-execution-plan/v1".into(),
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
    let mut invocation = vec![tool.path.to_str().ok_or("tool path")?.to_string()];
    invocation.extend_from_slice(argv);
    if tool.program != program
        || !tool.path.is_absolute()
        || tool.path.file_name().and_then(|s| s.to_str()) != Some(&tool.basename)
        || actual.executable != tool.path
        || actual.sha256 != tool.sha256
        || actual.argv != invocation
        || actual.cwd != root
        || actual.environment != *env
        || expected.is_none()
        || expected != Some(actual.stdout.trim_end())
        || actual.stdout.as_bytes() != actual.stdout_bytes
        || live
            && fs::read(&tool.path)
                .map(|b| crate::sha256(&b))
                .ok()
                .as_ref()
                != Some(&tool.sha256)
    {
        return Err("E_TOOL_BINDING: path/version/digest/environment observation mismatch".into());
    }
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
        digest(&v)
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
        .filter_map(|(id, operations)| match operations.as_slice() {
            [operation] => Some((id, operation.clone())),
            _ => None,
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
    if impact.schema != "chrono-filemap-impact/v6" || impact.delta != req.delta {
        return Err("E_IMPACT: identity".into());
    }
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
        let registered = r.config["environment"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| {
                req.candidate.root.join(i["location"].as_str().unwrap()) == tool.path
                    && i["sha256"] == tool.sha256
            })
            || tool
                .path
                .strip_prefix(&req.candidate.root)
                .ok()
                .and_then(|p| p.to_str())
                .is_some_and(|p| {
                    candidate.filemap["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["path"] == p)
                });
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
