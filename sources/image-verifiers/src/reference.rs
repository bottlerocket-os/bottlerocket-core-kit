//! Image reference construction.

/// Construct a notation-compatible image reference from name and digest.
/// Strips any existing tag or digest from the name and appends the new digest.
pub fn construct(name: &str, digest: &str) -> String {
    let mut name = name.to_string();
    if let Some(at) = name.find('@') {
        name.truncate(at);
    }
    if let Some(colon) = name.rfind(':') {
        if !name[colon..].contains('/') {
            name.truncate(colon);
        }
    }
    format!("{}@{}", name, digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("docker.io/library/nginx:latest", "sha256:abc", "docker.io/library/nginx@sha256:abc"; "with tag")]
    #[test_case("docker.io/library/nginx", "sha256:def", "docker.io/library/nginx@sha256:def"; "no tag")]
    #[test_case("localhost:5000/img:v1", "sha256:ghi", "localhost:5000/img@sha256:ghi"; "port and tag")]
    #[test_case("localhost:5000/img", "sha256:jkl", "localhost:5000/img@sha256:jkl"; "port no tag")]
    #[test_case("img@sha256:old", "sha256:new", "img@sha256:new"; "replace digest")]
    fn test_construct(name: &str, digest: &str, expected: &str) {
        assert_eq!(construct(name, digest), expected);
    }
}
