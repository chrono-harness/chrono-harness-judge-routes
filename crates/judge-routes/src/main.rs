fn main() -> std::process::ExitCode {
    let mut args = std::env::args_os().skip(1);
    let name = args.next();
    if name.as_deref().and_then(std::ffi::OsStr::to_str) == Some("--version") && args.len() == 0 {
        println!("chrono-judge-routes {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if let Some(
        operation @ ("evaluate-routes-v1"
        | "global-selection"
        | "collected-selection-v1"
        | "check-plan-receipts-v1"
        | "check-operation-receipt-v1"
        | "plan-context-v1"
        | "collection-plan-v1"
        | "full-operations-v1"
        | "source-plan-v1"
        | "unit-declarations-v1"
        | "retained-tool-v1"
        | "check-plan-v1"
        | "dispatch-plan-v1"
        | "check-scoped-plan-v1"
        | "scoped-operations-v1"
        | "prepare-scoped-plan-v1"),
    ) = name.as_deref().and_then(std::ffi::OsStr::to_str)
    {
        if args.next().is_some() {
            eprintln!("E_USAGE: {operation} reads a single registration query from stdin");
            return 2.into();
        }
        let input = std::io::stdin().lock();
        let output = std::io::stdout().lock();
        let result = match operation {
            "global-selection" => {
                chrono_judge_routes::selection::select_from_registries(input, output)
            }
            "collected-selection-v1" => std::env::current_dir()
                .map_err(|e| e.to_string())
                .and_then(|root| {
                    chrono_judge_routes::selection::select_collected(&root, input, output)
                }),
            command => {
                use chrono_judge_routes::plan_query;
                use std::io::Write;
                let reply = match command {
                    "evaluate-routes-v1" => chrono_judge_routes::judge(input),
                    "check-operation-receipt-v1" => plan_query::check_operation_receipt(input),
                    "check-plan-receipts-v1" => plan_query::check_plan_receipts(input),
                    "plan-context-v1" => plan_query::plan_context(input),
                    "source-plan-v1" => std::env::current_dir()
                        .map_err(|error| error.to_string())
                        .and_then(|root| chrono_judge_routes::source_plan::query(&root, input)),
                    "full-operations-v1" => plan_query::full_operations(input),
                    "retained-tool-v1" => plan_query::retained_tool(input),
                    "scoped-operations-v1" => plan_query::scoped_operations(input),
                    "prepare-scoped-plan-v1" => std::env::current_dir()
                        .map_err(|e| e.to_string())
                        .and_then(|root| plan_query::prepare_scoped_plan(&root, input)),
                    "check-scoped-plan-v1" => plan_query::check_scoped_plan(input),
                    "dispatch-plan-v1" => plan_query::dispatch_plan(input),
                    "check-plan-v1" => plan_query::check_plan(input),
                    "unit-declarations-v1" => plan_query::unit_declarations(input),
                    _ => plan_query::collection_plan(input),
                };
                reply.and_then(|reply| {
                    let mut output = output;
                    serde_json::to_writer(&mut output, &reply).map_err(|e| e.to_string())?;
                    output.flush().map_err(|e| e.to_string())
                })
            }
        };
        return match result {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                2.into()
            }
        };
    }
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
