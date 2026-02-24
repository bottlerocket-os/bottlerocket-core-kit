//! CLI argument parsing with Go-style flag support.

use argh::FromArgs;

/// Common arguments for image verifier binaries.
/// Containerd invokes verifiers with `-name <ref> -digest <sha256:...>`.
#[derive(FromArgs)]
#[argh(description = "Verify container image")]
pub struct Args {
    /// image reference
    #[argh(option, short = 'n')]
    pub name: String,
    /// image digest
    #[argh(option, short = 'd')]
    pub digest: String,
    /// media type of stdin JSON (unused, required by containerd interface)
    #[allow(dead_code)]
    #[argh(option)]
    stdin_media_type: Option<String>,
}

/// Convert Go-style single-hyphen flags to double-hyphen.
/// `-name` becomes `--name`, but `-n` and `--name` stay unchanged.
fn convert_go_style_args(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .map(|a| {
            if a.starts_with("-") && !a.starts_with("--") && a.len() > 2 {
                format!("-{}", a)
            } else {
                a
            }
        })
        .collect()
}

/// Parse CLI args, converting Go-style single-hyphen flags to double-hyphen.
/// Containerd uses `-name` instead of `--name`.
pub fn parse_go_style_args<T: FromArgs>() -> T {
    let args = convert_go_style_args(std::env::args().collect());
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    T::from_args(&strs[..1], &strs[1..]).unwrap_or_else(|e| {
        println!("{}", e.output);
        std::process::exit(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("-name", "--name"; "go style to double hyphen")]
    #[test_case("--name", "--name"; "double hyphen unchanged")]
    #[test_case("-n", "-n"; "short flag unchanged")]
    #[test_case("-", "-"; "single hyphen unchanged")]
    fn test_convert_go_style_args(input: &str, expected: &str) {
        let result = convert_go_style_args(vec![input.to_string()]);
        assert_eq!(result[0], expected);
    }
}
