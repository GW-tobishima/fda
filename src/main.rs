use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match forge_delivery_agent::cli::runner::run(env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}
