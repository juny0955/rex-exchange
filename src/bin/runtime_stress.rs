use std::{env, process};

#[path = "runtime_stress/mod.rs"]
mod runtime_stress;

use runtime_stress::{config::parse_config, runner::run, summary::print_usage};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let config = match parse_config(args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("오류: {message}");
            print_usage();
            process::exit(2);
        }
    };

    run(config);
}
