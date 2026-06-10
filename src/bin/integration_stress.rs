use std::{env, process};

#[path = "integration_stress/mod.rs"]
mod integration_stress;

use integration_stress::{config::parse_config, runner::run, summary::print_usage};

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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio 런타임 생성 실패");

    runtime.block_on(run(config));
}
