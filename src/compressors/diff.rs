use super::ansi::strip_ansi;

pub fn compress_diff(input: &str) -> String {
    let clean = strip_ansi(input);

    let mut output_lines = Vec::new();
    let mut current_file = String::new();
    let mut skipping_large_file = false;
    let mut skipped_additions = 0;
    let mut skipped_deletions = 0;

    let lockfile_names = [
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "poetry.lock",
        "go.sum",
        "composer.lock",
    ];

    for line in clean.lines() {
        if line.starts_with("diff --git ") {
            if skipping_large_file {
                output_lines.push(format!(
                    "    [Lockfile {} delta collapsed: +{}/-{} lines]",
                    current_file, skipped_additions, skipped_deletions
                ));
                skipping_large_file = false;
                skipped_additions = 0;
                skipped_deletions = 0;
            }

            current_file = line.split_whitespace().last().unwrap_or("").to_string();
            let is_lockfile = lockfile_names
                .iter()
                .any(|name| current_file.ends_with(name));
            let is_minified =
                current_file.ends_with(".min.js") || current_file.ends_with(".min.css");

            if is_lockfile || is_minified {
                skipping_large_file = true;
                output_lines.push(line.to_string());
                continue;
            }
        }

        if skipping_large_file {
            if line.starts_with('+') && !line.starts_with("+++") {
                skipped_additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                skipped_deletions += 1;
            }
            continue;
        }

        output_lines.push(line.to_string());
    }

    if skipping_large_file {
        output_lines.push(format!(
            "    [Lockfile {} delta collapsed: +{}/-{} lines]",
            current_file, skipped_additions, skipped_deletions
        ));
    }

    output_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_diff_lockfile() {
        let raw = r#"
diff --git a/Cargo.lock b/Cargo.lock
index 123456..789abc 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,5 +1,5 @@
 [[package]]
-name = "foo"
-version = "0.1.0"
+name = "foo"
+version = "0.2.0"
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
-fn old() {}
+fn new() {}
"#;
        let compressed = compress_diff(raw);
        assert!(compressed.contains("Lockfile b/Cargo.lock delta collapsed: +2/-2 lines"));
        assert!(compressed.contains("fn new() {}"));
    }
}
