pub mod ansi;
pub mod logs;
pub mod diff;
pub mod json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Auto,
    Log,
    Diff,
    Json,
    AnsiOnly,
}

pub fn compress(input: &str, content_type: ContentType) -> String {
    match content_type {
        ContentType::AnsiOnly => ansi::strip_ansi(input),
        ContentType::Log => logs::compress_logs(input),
        ContentType::Diff => diff::compress_diff(input),
        ContentType::Json => json::compress_json(input),
        ContentType::Auto => {
            let trimmed = input.trim_start();
            if trimmed.starts_with("diff --git ") || trimmed.starts_with("--- a/") {
                diff::compress_diff(input)
            } else if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                json::compress_json(input)
            } else {
                logs::compress_logs(input)
            }
        }
    }
}
