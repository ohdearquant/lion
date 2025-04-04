# LionForge IDE Testing Strategy

This document outlines the testing approach for the LionForge IDE project.

## 1. Goals

- Ensure correctness of both Rust backend and TypeScript/React frontend logic.
- Prevent regressions during development.
- Provide rapid, automated feedback to developers (including LLMs).
- Verify security-critical capability and policy integrations.
- Maintain high code quality.

## 2. Levels of Testing

### 2.1. Rust Backend Unit Tests (`#[test]`)

- **Location:** Within each Rust module (`mod tests { ... }`) or in
  `src-tauri/tests/`.
- **Scope:** Test individual functions, methods, and logic units within Tauri
  commands, state management, event handling, and utility modules.
- **Technique:**
  - Use standard `#[test]` and `assert!` macros.
  - **Mocking:** Use `mockall` or manual test doubles to mock dependencies,
    especially the `lion_runtime::Runtime` facade and its sub-managers
    (`PluginManager`, `CapabilityManager`, etc.). This isolates the logic under
    test.
  - Test success paths, error paths, and edge cases.
  - Verify correct interaction with mocks (expected method calls, arguments).
  - Test error mapping logic (converting internal errors to
    `Result<T, String>`).
- **Responsibility:** Implementer.
- **Verification:** QA, CI (`cargo test --all-targets`).

### 2.2. Rust Backend Integration Tests (`src-tauri/tests/`)

- **Location:** `src-tauri/tests/` directory.
- **Scope:** Test the interaction between different backend components,
  primarily focusing on sequences of Tauri commands and their effect on a _real_
  or near-real `lion_runtime` instance.
- **Technique:**
  - Set up a minimal `lion_runtime::Runtime` instance for the test suite or
    individual tests.
  - Programmatically invoke Tauri commands (if possible via a test harness,
    otherwise test the underlying Rust functions directly).
  - Assert on the state of the `Runtime` (e.g., loaded plugins, granted
    capabilities) after commands are executed.
  - Test sequences like Load Agent -> Grant Capability -> Check Status -> Unload
    Agent.
- **Responsibility:** Implementer (for key flows).
- **Verification:** QA, CI (`cargo test --all-targets`).

### 2.3. Frontend Component Tests (Vitest/Jest + RTL)

- **Location:** Alongside components (`*.test.tsx`) or in `src/tests/`.
- **Scope:** Test individual React components or small groups of interacting
  components.
- **Technique:**
  - Use Vitest or Jest as the test runner.
  - Use React Testing Library (`@testing-library/react`) for rendering and
    interacting with components.
  - **Mocking:** Mock Tauri API calls (`invoke`, `listen`) using `vi.fn()` or
    `jest.fn()`. Simulate backend responses and events. Mock state management
    hooks (e.g., Zustand `create` mock).
  - Assert on the rendered DOM output (`screen.getByText`, etc.).
  - Simulate user events (`userEvent.click`, `userEvent.type`) and assert on
    resulting state changes or API calls.
- **Responsibility:** Implementer (recommended for complex components).
- **Verification:** QA (optional review), CI (`npm test` or `yarn test`).

### 2.4. End-to-End Tests (Future - Phase 5+)

- **Location:** Separate test suite (`e2e/`).
- **Scope:** Test full user flows through the compiled Tauri application.
- **Technique:** Use Tauri's WebDriver (`tauri-driver`), Playwright, or Cypress
  (if running against a dev web server). Script interactions like opening
  projects, clicking buttons, filling forms, verifying UI updates.
- **Responsibility:** Dedicated QA effort or Implementer for critical paths.
- **Verification:** CI (potentially separate, longer-running job).

## 3. Automated Feedback Loop for LLMs

- **Implementer LLM:**
  - Instructed to run `cargo test` after modifying Rust code.
  - Instructed to run frontend tests (`npm test`) after modifying frontend code.
  - **MUST** include the pass/fail status and summary output of these tests in
    its completion message or commit summary.
- **QA LLM:**
  - **MUST** execute `cargo test --all-targets` on the provided code branch.
  - **MUST** analyze the test output for failures.
  - **MUST** review the implemented test code (`#[test]` functions) against the
    design requirements to assess coverage and correctness.
  - (Optional) Execute frontend tests if configured.
  - Provides feedback explicitly stating whether tests pass and highlighting any
    gaps or failures.

## 4. Continuous Integration (CI)

- Set up GitHub Actions (or similar).
- **Jobs:**
  - Rust Check (`cargo fmt --check`,
    `cargo clippy --all-targets -- -D warnings`).
  - Rust Test (`cargo test --all-targets`).
  - Frontend Check (`npm ci` or `yarn install`, `npm run lint`,
    `npm run build`).
  - Frontend Test (`npm test` or `yarn test`).
- All jobs **MUST** pass for a PR to be mergeable.

## 5. Quality Gates

- **Implementer:** Code is not considered complete until associated
  unit/integration tests are written and passing locally.
- **QA:** Code is not approved until automated tests pass in CI and manual
  review confirms adequate test coverage and adherence to standards.
- **Orchestrator:** Does not merge code or proceed to the next dependent task if
  tests are failing based on Implementer or QA reports.
