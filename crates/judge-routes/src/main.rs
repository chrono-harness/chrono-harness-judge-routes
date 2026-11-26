//! Public command selection for the installed routes executable.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum Query {
    Evaluate,
    GlobalSelection,
    CollectedSelection,
    PlanReceipts,
    OperationReceipt,
    PlanContext,
    CollectionPlan,
    FullOperations,
    SourcePlan,
    UnitAssignments,
    UnitDeclarations,
    RetainedTool,
    CheckPlan,
    DispatchPlan,
    ScopedPlan,
    ScopedOperations,
    PrepareScopedPlan,
}

pub enum Command { Version, Query(Query), Usage, Judge }

pub open spec fn query_name(query: Query) -> Seq<char> {
    match query {
        Query::Evaluate => "evaluate-routes-v1"@,
        Query::GlobalSelection => "global-selection"@,
        Query::CollectedSelection => "collected-selection-v1"@,
        Query::PlanReceipts => "check-plan-receipts-v1"@,
        Query::OperationReceipt => "check-operation-receipt-v1"@,
        Query::PlanContext => "plan-context-v1"@,
        Query::CollectionPlan => "collection-plan-v1"@,
        Query::FullOperations => "full-operations-v1"@,
        Query::SourcePlan => "source-plan-v1"@,
        Query::UnitAssignments => "check-unit-assignments-v1"@,
        Query::UnitDeclarations => "unit-declarations-v1"@,
        Query::RetainedTool => "retained-tool-v1"@,
        Query::CheckPlan => "check-plan-v1"@,
        Query::DispatchPlan => "dispatch-plan-v1"@,
        Query::ScopedPlan => "check-scoped-plan-v1"@,
        Query::ScopedOperations => "scoped-operations-v1"@,
        Query::PrepareScopedPlan => "prepare-scoped-plan-v1"@,
    }
}

pub open spec fn known(name: Seq<char>) -> bool {
    name == "evaluate-routes-v1"@
        || name == "global-selection"@
        || name == "collected-selection-v1"@
        || name == "check-plan-receipts-v1"@
        || name == "check-operation-receipt-v1"@
        || name == "plan-context-v1"@
        || name == "collection-plan-v1"@
        || name == "full-operations-v1"@
        || name == "source-plan-v1"@
        || name == "check-unit-assignments-v1"@
        || name == "unit-declarations-v1"@
        || name == "retained-tool-v1"@
        || name == "check-plan-v1"@
        || name == "dispatch-plan-v1"@
        || name == "check-scoped-plan-v1"@
        || name == "scoped-operations-v1"@
        || name == "prepare-scoped-plan-v1"@
}

/// Empty input represents an absent or non-UTF-8 name. The judge receives the original OS arguments.
pub fn command(name: &str, extra: bool) -> (choice: Command)
    ensures
        (choice is Version) == (name@ == "--version"@ && !extra),
        (choice is Query) == (known(name@) && !extra),
        (choice is Usage) == (known(name@) && extra),
        (choice is Judge) == (!known(name@) && (name@ != "--version"@ || extra)),
        choice matches Command::Query(query) ==> name@ == query_name(query),
{
    proof {
        reveal_strlit("--version");
        reveal_strlit("evaluate-routes-v1");
        reveal_strlit("global-selection");
        reveal_strlit("collected-selection-v1");
        reveal_strlit("check-plan-receipts-v1");
        reveal_strlit("check-operation-receipt-v1");
        reveal_strlit("plan-context-v1");
        reveal_strlit("collection-plan-v1");
        reveal_strlit("full-operations-v1");
        reveal_strlit("source-plan-v1");
        reveal_strlit("check-unit-assignments-v1");
        reveal_strlit("unit-declarations-v1");
        reveal_strlit("retained-tool-v1");
        reveal_strlit("check-plan-v1");
        reveal_strlit("dispatch-plan-v1");
        reveal_strlit("check-scoped-plan-v1");
        reveal_strlit("scoped-operations-v1");
        reveal_strlit("prepare-scoped-plan-v1");
    }
    let query = if <&str>::exec_eq(name, "evaluate-routes-v1") {
        Query::Evaluate
    } else if <&str>::exec_eq(name, "global-selection") {
        Query::GlobalSelection
    } else if <&str>::exec_eq(name, "collected-selection-v1") {
        Query::CollectedSelection
    } else if <&str>::exec_eq(name, "check-plan-receipts-v1") {
        Query::PlanReceipts
    } else if <&str>::exec_eq(name, "check-operation-receipt-v1") {
        Query::OperationReceipt
    } else if <&str>::exec_eq(name, "plan-context-v1") {
        Query::PlanContext
    } else if <&str>::exec_eq(name, "collection-plan-v1") {
        Query::CollectionPlan
    } else if <&str>::exec_eq(name, "full-operations-v1") {
        Query::FullOperations
    } else if <&str>::exec_eq(name, "source-plan-v1") {
        Query::SourcePlan
    } else if <&str>::exec_eq(name, "check-unit-assignments-v1") {
        Query::UnitAssignments
    } else if <&str>::exec_eq(name, "unit-declarations-v1") {
        Query::UnitDeclarations
    } else if <&str>::exec_eq(name, "retained-tool-v1") {
        Query::RetainedTool
    } else if <&str>::exec_eq(name, "check-plan-v1") {
        Query::CheckPlan
    } else if <&str>::exec_eq(name, "dispatch-plan-v1") {
        Query::DispatchPlan
    } else if <&str>::exec_eq(name, "check-scoped-plan-v1") {
        Query::ScopedPlan
    } else if <&str>::exec_eq(name, "scoped-operations-v1") {
        Query::ScopedOperations
    } else if <&str>::exec_eq(name, "prepare-scoped-plan-v1") {
        Query::PrepareScopedPlan
    } else {
        return if !extra && <&str>::exec_eq(name, "--version") { Command::Version } else { Command::Judge };
    };
    if extra { Command::Usage } else { Command::Query(query) }
}

}
#[cfg(not(verus_keep_ghost))]
fn write_reply(
    reply: Result<serde_json::Value, String>,
    mut output: impl std::io::Write,
) -> Result<(), String> {
    serde_json::to_writer(&mut output, &reply?).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::process::ExitCode {
    use chrono_judge_routes::plan_query;
    let mut args = std::env::args_os().skip(1);
    let name = args.next();
    let name = name
        .as_deref()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    let query = match command(name, args.len() != 0) {
        Command::Version => {
            println!("chrono-judge-routes {}", env!("CARGO_PKG_VERSION"));
            return std::process::ExitCode::SUCCESS;
        }
        Command::Usage => {
            eprintln!("E_USAGE: {name} reads a single registration query from stdin");
            return 2.into();
        }
        Command::Judge => return judge_command(),
        Command::Query(query) => query,
    };
    let input = std::io::stdin().lock();
    let output = std::io::stdout().lock();
    let result = match query {
        Query::GlobalSelection => {
            chrono_judge_routes::selection::select_from_registries(input, output)
        }
        Query::CollectedSelection => {
            std::env::current_dir()
                .map_err(|e| e.to_string())
                .and_then(|root| {
                    chrono_judge_routes::selection::select_collected(&root, input, output)
                })
        }

        Query::Evaluate => write_reply(chrono_judge_routes::judge(input), output),
        Query::OperationReceipt => write_reply(plan_query::check_operation_receipt(input), output),
        Query::PlanReceipts => write_reply(plan_query::check_plan_receipts(input), output),
        Query::PlanContext => write_reply(plan_query::plan_context(input), output),
        Query::SourcePlan => write_reply(
            std::env::current_dir()
                .map_err(|error| error.to_string())
                .and_then(|root| chrono_judge_routes::source_plan::query(&root, input)),
            output,
        ),
        Query::FullOperations => write_reply(plan_query::full_operations(input), output),
        Query::RetainedTool => write_reply(plan_query::retained_tool(input), output),
        Query::ScopedOperations => write_reply(plan_query::scoped_operations(input), output),
        Query::PrepareScopedPlan => write_reply(
            std::env::current_dir()
                .map_err(|e| e.to_string())
                .and_then(|root| plan_query::prepare_scoped_plan(&root, input)),
            output,
        ),
        Query::ScopedPlan => write_reply(plan_query::check_scoped_plan(input), output),
        Query::DispatchPlan => write_reply(plan_query::dispatch_plan(input), output),
        Query::CheckPlan => write_reply(plan_query::check_plan(input), output),
        Query::UnitAssignments => write_reply(plan_query::check_unit_assignments(input), output),
        Query::UnitDeclarations => write_reply(plan_query::unit_declarations(input), output),
        Query::CollectionPlan => write_reply(plan_query::collection_plan(input), output),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            2.into()
        }
    }
}

#[cfg(not(verus_keep_ghost))]
fn judge_command() -> std::process::ExitCode {
    use std::os::unix::process::CommandExt;
    let invoked = std::env::args_os().next().map(std::path::PathBuf::from);
    let executable = match invoked.as_deref() {
        Some(path) if path.components().count() > 1 => std::path::absolute(path),
        _ => std::env::current_exe(),
    };
    let program = match executable {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{error}");
            return 2.into();
        }
    };
    let error = std::process::Command::new(chrono_judge_routes::runner_program())
        .args([
            "judge-command-v1",
            "chrono-judge-routes",
            env!("CARGO_PKG_VERSION"),
        ])
        .arg(program)
        .args(["evaluate-routes-v1", "E_ROUTES", "--inherited"])
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("start chrono-harness judge-command-v1: {error}");
    2.into()
}
