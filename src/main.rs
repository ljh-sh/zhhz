use std::process::ExitCode;

fn main() -> ExitCode {
    match zhhz::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
