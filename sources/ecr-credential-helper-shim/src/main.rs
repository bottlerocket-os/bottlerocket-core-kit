use std::env;
use std::process::{self, Command};

fn main() {
    let godebug = env::var("GODEBUG")
        .unwrap_or_default()
        .replace("fips140=only", "fips140=on");

    let status = Command::new("/usr/libexec/docker-credential-ecr-login")
        .args(env::args().skip(1))
        .env("GODEBUG", &godebug)
        .status();

    match status {
        Ok(s) => process::exit(s.code().unwrap_or(1)),
        Err(err) => {
            eprintln!("Failed to exec /usr/libexec/docker-credential-ecr-login: {err}");
            process::exit(1);
        }
    }
}
