# Repository Guidelines

## Project Structure & Modules
- Workspace root: Rust monorepo with three crates.
  - `dialog_lib/` — core Rust library, business logic and tests.
  - `dialog_cli/` — CLI using `dialog_lib`.
  - `dialog_uniffi/` — UniFFI wrapper for Swift/iOS.
- iOS app: `ios/` (SwiftUI app consuming `dialog_uniffi`).
- Docs & plans: `docs/`, plus focused MD guides in the root.
- Examples: `examples/` (library usage).
- Scripts: `justfile`, `rebuild.sh`, `build-uniffi-package.sh`, test scripts in root.

## Build, Test, Run
- Build all (release): `just build` → `target/release/` artifacts.
- Run CLI: `just cli -- list --limit 10` or `cargo run -p dialog_cli -- list`.
- iOS smoke build: `just ios` (builds UniFFI package, then Xcode build).
- Tests: `just test` or `cargo test` (unit + integration for `dialog_lib`).
- Format/lint: `just check` (runs `cargo fmt` and `cargo clippy`).

## Coding Style & Naming
- Rust: 4-space indent, `snake_case` functions/modules, `CamelCase` types, `SCREAMING_SNAKE_CASE` consts.
  - Prefer ` anyhow/thiserror ` patterns for errors (`thiserror` is already in workspace).
  - Keep public API in `dialog_lib` minimal; re-export types close to where used.
- Swift: lowerCamelCase for vars/functions; PascalCase for types. Keep UniFFI-facing APIs stable.
- Formatting: use `cargo fmt`; fix clippy warnings when reasonable.

## Testing Guidelines
- Rust unit tests live beside code under `#[cfg(test)]`; integration tests in `dialog_lib/tests/`.
- Name tests descriptively (e.g., `test_parse_hashtags`); prefer small, deterministic tests.
- Async: use `tokio::test` where needed; avoid network in unit tests.
- Quick E2E: optional `./test_quick.sh` and `./test_cli_persistence.sh` (spawns local relay).

## Commit & PR Guidelines
- Commit style: Conventional Commits (observed: `feat`, `fix`, `docs`, `refactor`, `ci`). Example: `feat(cli): add watch mode`.
- Scope PRs narrowly; include:
  - Summary, motivation, and screenshots/logs when applicable.
  - Commands to reproduce (e.g., `just cli -- list`).
  - Linked issues and notes on risk/rollback.

## Security & Configuration
- CLI uses env vars: `DIALOG_NSEC` (required), `DIALOG_RELAY` (default local), `DIALOG_DATA_DIR`.
- Do not commit secrets. Prefer local `.envrc` or shell exports; never hardcode keys in code.
