## **Overall Project Style Guide**

This document sets forth **global** coding and repository practices for the Lion
microkernel project, ensuring consistent quality and maintainability.

### **1. Code Structure & Modularity**

- **Project Organization**:
  - Use a **modular approach**: separate core kernel logic (`kernel_state.rs`,
    `ipu.rs`, etc.) from higher-level or optional components (e.g., logging ring
    buffer, concurrency features).
  - Place supporting files (e.g., tests, docs) in well-defined directories:
    - `/tests/` for integration tests,
    - `/docs/` or `/design/` for architectural diagrams or design notes.

- **Cargo Workspaces** (Optional):
  - If we eventually split the codebase into multiple crates (e.g., kernel
    crate, userland crate), define a top-level `Cargo.toml` with `[workspace]`.
  - For smaller scope, a single crate is fine.

### **2. Coding Conventions & Naming**

- **Rust Edition**:
  - Target **Rust 2021** (or stable versions).
- **Naming Conventions**:
  - **Types & Enums**: `PascalCase` (e.g., `KernelState`, `IpuControlBlock`).
  - **Functions & Methods**: `snake_case` (e.g., `create_ipu`, `run_ipu`).
  - **Constants**: `UPPER_CASE_WITH_UNDERSCORES`.
  - **Modules**: typically `snake_case`.
- **Visibility**:
  - Use `pub(crate)` or `pub(in crate)` for items not needed outside the current
    crate. Keep the kernel’s internal details private where possible.

### **3. Coding Style & Linting**

- **Formatting**:
  - Enforce `cargo fmt` with default style.
- **Clippy**:
  - Run `cargo clippy -- -D warnings` to catch common mistakes.
  - Address or document any clippy suggestions that are intentionally bypassed.
- **Error Handling & Panics**:
  - Where possible, prefer returning `Result<T, E>` in user-facing APIs.
  - In the microkernel internal code, if a panic is truly fatal, ensure we
    handle it carefully (or log it). For Stage 1, we catch user-level IPU panics
    but keep kernel-level panics minimal.

### **4. Git & GitHub Hygiene**

- **Branching Strategy**:
  - Option A: **Feature branches** for each major task (e.g.,
    “phase1-environment”, “phase2-datastructures”), merged into `main` after
    review.
  - Option B: **Trunk-based** with small commits directly in `main`, as long as
    CI passes.
- **Commit Messages**:
  - Use concise yet descriptive messages.
  - For multi-commit merges (e.g., finishing a Phase), mention “Completes
    Stage 1, Phase 1.2: Data Structures.”
- **Pull Requests**:
  - Encourage team reviews on PRs.
  - CI checks must pass (build + test).
  - Keep each PR focused on a single topic or phase.
- **Tagging**:
  - Tag stable milestones (e.g., `stage1-complete`) to mark versioned points.

### **5. Documentation & Comments**

- **Doc Comments** (`///`) on types and key methods:
  - Summarize purpose, usage, and potential edge cases.
  - Provide code snippets if it clarifies usage.
- **README** in the root:
  - High-level overview of the microkernel, build/run instructions, link to
    advanced docs.
- **Design Docs**:
  - If major changes are introduced (like concurrency or HPC expansions), keep a
    short design doc in `/docs/` explaining the rationale and approach.

### **6. Testing Philosophy**

- **Unit Tests**:
  - Place them alongside code in `#[cfg(test)]` blocks or in dedicated test
    files under `/tests/`.
  - Name them clearly (e.g., `test_create_ipu_panic()`).
- **Integration Tests**:
  - Larger end-to-end scenarios. Possibly named `tests/test_schedule_all.rs` or
    similar.
- **CI**:
  - Run `cargo test` on all PRs.
  - Optionally run `cargo fmt --check` and `cargo clippy -- -D warnings`.

### **7. Logging & Debugging**

- **Single-thread**:
  - Basic `println!` or ring buffer for logs is sufficient in early stages.
- **Multi-thread** (Stage 2+):
  - Transition to a thread-safe logging approach or a known Rust logging crate.
- **Crash Info**:
  - Collect user-level panics in IPUs with a dedicated field or logs; treat
    kernel-level panics as fatal.

### **8. Resource Usage & HPC Expansions**

- **Scalability**:
  - Plan for HPC expansions in Stage 2 or beyond. Keep data structures
    concurrency-friendly if possible (e.g., no global mutable singletons that
    hamper multi-core).
- **Performance**:
  - Use benchmarks (e.g., microbench for scheduling overhead) once concurrency
    arrives.
  - For single-thread MVP, simple checks suffice.

**This Style Guide** ensures the code remains consistent, readable, testable,
and poised for expansions in concurrency or HPC. Keep it updated as the project
grows.