use regex::Regex;
use std::sync::LazyLock;

static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\x1B\[[0-?]*[ -/]*[@-~])|(\x1B\][^\x07\x1B]*(\x07|\x1B\\))").unwrap()
});

pub fn strip_ansi(input: &str) -> String {
    let clean = ANSI_REGEX.replace_all(input, "");
    clean.replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let raw = "\x1b[31;1mError:\x1b[0m Failed to build \x1b[32mpackage\x1b[0m\r\n";
        assert_eq!(strip_ansi(raw), "Error: Failed to build package\n");
    }
}
