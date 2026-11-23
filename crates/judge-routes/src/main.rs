fn main() -> std::process::ExitCode {
    let mut args = std::env::args_os().skip(1);
    let name = args.next();
    if let Some(operation @ ("global-selection" | "collected-selection-v1")) =
        name.as_deref().and_then(std::ffi::OsStr::to_str)
    {
        if args.next().is_some() {
            eprintln!("E_USAGE: {operation} reads a single registration query from stdin");
            return 2.into();
        }
        let input = std::io::stdin().lock();
        let output = std::io::stdout().lock();
        let result = if operation == "global-selection" {
            chrono_judge_routes::selection::select_from_registries(input, output)
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())
                .and_then(|root| {
                    chrono_judge_routes::selection::select_collected(&root, input, output)
                })
        };
        return match result {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                2.into()
            }
        };
    }
    std::process::ExitCode::from(chrono_harness::wire::judge_main(
        "chrono-judge-routes",
        env!("CARGO_PKG_VERSION"),
        chrono_judge_routes::judge,
    ))
}
