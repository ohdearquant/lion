# LionForge Rust Coding Style Guide

This guide outlines the coding standards for the Rust backend of the LionForge
IDE. Adherence is mandatory and checked by QA and CI.

## 1. Formatting

- **Tool:** `rustfmt` (via `cargo fmt`)
- **Requirement:** All code **MUST** be formatted using the default `rustfmt`
  configuration included with the stable Rust toolchain. Run
  `cargo fmt --all --check` locally and ensure it passes before committing. CI
  will enforce this.

## 2. Linting

- **Tool:** `clippy` (via `cargo clippy`)
- **Requirement:** All code **MUST** pass
  `cargo clippy --all-targets -- -D warnings`. Resolve all warnings flagged by
  clippy. Do not use `#![allow(clippy::...)]` at the crate level unless
  explicitly approved for a specific, well-justified reason. Use
  `#\[allow(...)]` sparingly on specific items with a clear comment explaining
  why.

## 3. Rust Edition

- Use **Rust 2021 Edition**. Ensure `edition = "2021"` is set in all
  `Cargo.toml` files.

## 4. Naming Conventions

- **Modules, Crates, Files:** `snake_case` (e.g., `runtime_integration.rs`,
  `lion_proxy_plugin/`)
- **Types (Structs, Enums, Traits):** `PascalCase` (e.g., `WorkflowManager`,
  `PluginState`)
- **Functions, Methods, Variables:** `snake_case` (e.g., `fn list_agents`,
  `let agent_count = ...`)
- **Constants, Statics:** `UPPER_SNAKE_CASE` (e.g.,
  `const DEFAULT_TIMEOUT: u64 = 5000;`)
- **Type Parameters:** Single uppercase letter (e.g., `<T>`) or descriptive
  `PascalCase` if complex (e.g., `<S: StorageBackend>`).

## 5. Modularity

- Organize code into logical modules (e.g., `commands`, `state`, `events`).
- Keep modules focused on a single responsibility.
- Use `pub(crate)` for internal visibility where appropriate. Expose only the
  necessary public API from each module/crate.

## 6. Error Handling

- **Primary:** Use `anyhow::Result<T>` or `Result<T, SpecificError>` for
  functions that can fail.
- **Custom Errors:** Define specific error enums using `thiserror` for distinct
  error conditions within modules (e.g., `PluginManagerError`,
  `StateMachineError`).
- **Mapping:** Convert specific errors into higher-level errors (e.g.,
  `StateMachineError` into a `String` for Tauri command results) using `map_err`
  or `?` with `From` implementations where appropriate.
- **Avoid `panic!`:** Do not use `panic!`, `unwrap()`, or `expect()` in
  production code paths. Use them only in tests for conditions that _must_ hold
  or during initial setup where failure is unrecoverable. Prefer `anyhow::bail!`
  or returning `Err`.

## 7. Concurrency

- **Runtime:** Use `tokio` as the primary async runtime.
- **Shared State:** Prefer `Arc<RwLock<T>>` (from `tokio::sync`) or
  `Arc<Mutex<T>>` (from `tokio::sync`) for shared mutable state accessed across
  async tasks. Use `parking_lot` variants _only_ if blocking within async code
  is strictly necessary and understood (rarely needed).
- **Channels:** Use `tokio::sync::mpsc` for message passing between async tasks,
  `tokio::sync::broadcast` for 1-to-N event notifications.
- **Blocking Code:** If interfacing with blocking code (e.g., some file I/O,
  CPU-intensive tasks), use `tokio::task::spawn_blocking`.

## 8. Dependencies

- Keep dependencies minimal.
- Use specific versions in `Cargo.toml` (e.g., `1.0.163` not `1.0`). Run
  `cargo update` consciously.
- Enable only necessary features for dependencies.

## 9. Logging & Tracing

- Use the `tracing` crate for logging and potential distributed tracing.
- Use structured logging macros (e.g., `info!(param = value, "Message")`).
- Follow log levels consistently (TRACE, DEBUG, INFO, WARN, ERROR).

## 10. Documentation

- Write `///` doc comments for all public functions, structs, enums, traits, and
  modules.
- Explain _why_ code exists, not just _what_ it does, especially for complex
  logic.
- Include small usage examples in doc comments where helpful.

## 11. Unsafe Code

- Avoid `unsafe` blocks unless absolutely necessary and unavoidable (e.g., FFI).
- If `unsafe` is used, provide a detailed `// SAFETY:` comment explaining why
  it's necessary and justifying its safety based on invariants.

## 12. Code Comments

- Use `//` for implementation details, rationale, TODOs, or FIXMEs.
- Keep comments concise and up-to-date with the code.
