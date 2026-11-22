fn main() -> std::process::ExitCode {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("global-selection")) {
        if args.next().is_some() {
            eprintln!("E_USAGE: global-selection reads a single registration query from stdin");
            return 2.into();
        }
        return match chrono_judge_routes::selection::select_from_registries(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        ) {
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
