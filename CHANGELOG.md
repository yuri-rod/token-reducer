# Changelog

## 0.1.2 - 2026-10-28

- Release generated stack-frame omission summaries after each log reduction instead
  of leaking their memory during repeated library or MCP calls.
- Iterate log and diff input directly without collecting an intermediate line vector.
- Add an allocation regression that preserves the exact reduced output and checks
  both intermediate and trailing stack-frame summaries.

## 0.1.1 - 2026-10-27

- Preserve child-command exit codes and Git failure diagnostics in the CLI.
- Report failed MCP command executions as tool errors.
- Keep TypeScript failure lines when collapsing passing-test output.
- Report the package version during MCP initialization.
- Add command-failure regressions and document reproducible examples, lossy compression behavior and local command-execution limits.
