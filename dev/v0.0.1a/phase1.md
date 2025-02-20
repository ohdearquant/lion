Below is an **exhaustively detailed set of instructions** for **Phase 1** of the lion project, as described in our broader Developer Guide. This document focuses on **exact** steps and expectations for setting up the Rust workspace, defining core primitives, creating a minimal CLI, and validating everything through tests. All references to coding style, concurrency, security, etc., come from the overarching lion Developer Guide.

---

# Phase 1 – Workspace Setup & Core Primitives

## 1. **High-Level Objectives**
1. Create a **Rust workspace** with two crates:
   - **`agentic_core`** (library crate for microkernel logic and data structures)
   - **`agentic_cli`** (binary crate for a simple command-line interface)
2. Define and test **core primitives**:
   - `ElementData` – a base “trackable entity”
   - `Pile<T>` – a concurrency-safe container for storing objects by `Uuid`
   - `Progression` – an ordered sequence referencing items in a Pile
3. Implement a **minimal in-memory store** to track elements.
4. Provide **basic CLI commands** to create and list stored elements.
5. Validate the entire setup using:
   - **Unit tests** (for each primitive)
   - **Integration/CLI tests** (for the workflow of creating and listing elements)
6. Mark phase completion as `v0.0.1a-phase1`.

---

## 2. **Technical Requirements & Outcomes**

1. **Project Layout**  
   - A top-level `Cargo.toml` referencing the two crates (`agentic_core`, `agentic_cli`) in a `[workspace]` section.
   - `agentic_core` includes modules: `element.rs`, `pile.rs`, `progression.rs`, and possibly `store.rs`.
   - `agentic_cli` includes a `main.rs` that provides at least two commands.

2. **Core Data Structures**  
   - **`ElementData`**:  
     - Fields: `id: Uuid`, `created_at: DateTime<Utc>`, `metadata: serde_json::Value`.  
     - Constructor: `new(metadata: Value) -> Self`.
   - **`Pile<T>`**:  
     - Internally: `Arc<Mutex<HashMap<Uuid, T>>>`.  
     - Methods: `insert(&self, id: Uuid, item: T)`, `get(&self, id: &Uuid) -> Option<T>` (if `T: Clone`), `list_ids(&self) -> Vec<Uuid>`.
   - **`Progression`**:
     - A struct with a simple `Vec<Uuid>` to track order of items.
     - Methods to push an ID or retrieve the sequence.

3. **In-Memory Store**  
   - A struct `InMemoryStore` that wraps or composes a `Pile<ElementData>`.
   - Methods:
     - `create_element(&self, data: ElementData) -> Uuid`
     - `get_element(&self, &Uuid) -> Option<ElementData>`
     - `list_element_ids(&self) -> Vec<Uuid>`

4. **CLI**  
   - `create-element --metadata '{...}'` → parse JSON, create new `ElementData`, store it, print the new ID.
   - `list-elements` → fetch all IDs from store, print them line-by-line.

5. **Validation**  
   - All unit tests pass (`cargo test`).
   - `cargo fmt`, `cargo clippy` yield no errors.
   - Manual CLI test: creating an element and listing it.

**Expected Outcome**  
- A fully working Rust workspace with minimal concurrency-safe data structures, a very simple ephemeral store, and a CLI to confirm basic operations.  
- Tag the final commit as `v0.0.1a-phase1`.

---

## 3. **Step-by-Step Implementation**

This phase can be broken into **four** major tasks: **(A) Workspace Setup**, **(B) Core Primitives** (`agentic_core`), **(C) In-Memory Store & CLI**, **(D) Testing & Validation**.

### 3.A. **Workspace Setup**

1. **Initialize Repository**  
   - If this is the start of the project, run:
     ```bash
     git init lion
     cd lion
     ```
     (You may already have a repo from prior planning—adjust accordingly.)

2. **Create `agentic_core` Library Crate**  
   ```bash
   cargo new agentic_core --lib
   ```
   This generates a `Cargo.toml` and `src/lib.rs` in the `agentic_core` folder.

3. **Create `agentic_cli` Binary Crate**  
   ```bash
   cargo new agentic_cli --bin
   ```
   This generates a separate `Cargo.toml` and `src/main.rs` in the `agentic_cli` folder.

4. **Add Workspace `Cargo.toml`**  
   At the root (`lion/`), create a `Cargo.toml` with:
   ```toml
   [workspace]
   members = [
     "agentic_core",
     "agentic_cli"
   ]
   ```

5. **Check Build**  
   ```bash
   cargo build
   ```
   Confirm no errors. If any arise, verify the `[workspace]` block is correct.

6. **Lint & Format**  
   ```bash
   cargo fmt
   cargo clippy
   ```
   Fix any formatting or lint suggestions as per the Developer Guide.

**Result**: A workspace with two crates recognized by the top-level `Cargo.toml`.

### 3.B. **Core Primitives (`agentic_core`)**

Create or modify files in `agentic_core/src/`:

1. **`element.rs`**  
   ```rust
   // agentic_core/src/element.rs
   use uuid::Uuid;
   use chrono::{DateTime, Utc};
   use serde::{Serialize, Deserialize};
   use serde_json::Value;

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ElementData {
       pub id: Uuid,
       pub created_at: DateTime<Utc>,
       pub metadata: Value,
   }

   impl ElementData {
       pub fn new(metadata: Value) -> Self {
           Self {
               id: Uuid::new_v4(),
               created_at: Utc::now(),
               metadata,
           }
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use serde_json::json;

       #[test]
       fn test_element_creation() {
           let meta = json!({ "title": "Test Element" });
           let elem = ElementData::new(meta.clone());
           assert_eq!(elem.metadata, meta);
           assert_ne!(elem.id, Uuid::nil());
       }
   }
   ```

2. **`pile.rs`**  
   ```rust
   // agentic_core/src/pile.rs
   use std::collections::HashMap;
   use std::sync::{Arc, Mutex};
   use uuid::Uuid;

   #[derive(Debug, Clone)]
   pub struct Pile<T> {
       inner: Arc<Mutex<HashMap<Uuid, T>>>,
   }

   impl<T> Pile<T> {
       pub fn new() -> Self {
           Self {
               inner: Arc::new(Mutex::new(HashMap::new())),
           }
       }

       pub fn insert(&self, id: Uuid, item: T) {
           let mut guard = self.inner.lock().unwrap();
           guard.insert(id, item);
       }

       pub fn get(&self, id: &Uuid) -> Option<T>
       where
           T: Clone,
       {
           let guard = self.inner.lock().unwrap();
           guard.get(id).cloned()
       }

       pub fn list_ids(&self) -> Vec<Uuid> {
           let guard = self.inner.lock().unwrap();
           guard.keys().cloned().collect()
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use std::thread;

       #[test]
       fn test_pile_insert_retrieve() {
           let pile = Pile::new();
           let id = Uuid::new_v4();
           pile.insert(id, "test_data".to_string());
           let retrieved = pile.get(&id);
           assert_eq!(retrieved, Some("test_data".to_string()));
       }

       #[test]
       fn test_pile_concurrency() {
           let pile = Pile::new();
           let handles: Vec<_> = (0..10).map(|_| {
               let p = pile.clone();
               thread::spawn(move || {
                   let id = Uuid::new_v4();
                   p.insert(id, format!("val-{}", id));
               })
           }).collect();

           for h in handles { h.join().unwrap(); }

           // Just confirm we have 10 items now
           let all_ids = pile.list_ids();
           assert_eq!(all_ids.len(), 10);
       }
   }
   ```

3. **`progression.rs`**  
   ```rust
   // agentic_core/src/progression.rs
   use uuid::Uuid;

   #[derive(Debug, Default)]
   pub struct Progression {
       steps: Vec<Uuid>,
   }

   impl Progression {
       pub fn new() -> Self {
           Self { steps: Vec::new() }
       }

       pub fn push(&mut self, id: Uuid) {
           self.steps.push(id);
       }

       pub fn list(&self) -> &[Uuid] {
           &self.steps
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use uuid::Uuid;

       #[test]
       fn test_progression_push_list() {
           let mut prog = Progression::new();
           let id1 = Uuid::new_v4();
           let id2 = Uuid::new_v4();

           prog.push(id1);
           prog.push(id2);

           let all = prog.list();
           assert_eq!(all.len(), 2);
           assert_eq!(all[0], id1);
           assert_eq!(all[1], id2);
       }
   }
   ```

4. **`lib.rs`**  
   ```rust
   // agentic_core/src/lib.rs
   pub mod element;
   pub mod pile;
   pub mod progression;

   // Future modules: store, orchestrator, plugin_manager, etc.
   ```

Now, run `cargo test -p agentic_core`. Confirm all tests pass.

### 3.C. **In-Memory Store & CLI**

1. **`store.rs`** in `agentic_core`  
   ```rust
   // agentic_core/src/store.rs
   use crate::element::ElementData;
   use crate::pile::Pile;
   use uuid::Uuid;

   pub struct InMemoryStore {
       elements: Pile<ElementData>,
   }

   impl InMemoryStore {
       pub fn new() -> Self {
           Self { elements: Pile::new() }
       }

       pub fn create_element(&self, elem: ElementData) -> Uuid {
           let id = elem.id;
           self.elements.insert(id, elem);
           id
       }

       pub fn get_element(&self, id: &Uuid) -> Option<ElementData> {
           self.elements.get(id)
       }

       pub fn list_element_ids(&self) -> Vec<Uuid> {
           self.elements.list_ids()
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use serde_json::json;

       #[test]
       fn test_store_create_element() {
           let store = InMemoryStore::new();
           let data = ElementData::new(json!({ "hello": "world" }));
           let id = store.create_element(data);
           let retrieved = store.get_element(&id);
           assert!(retrieved.is_some());
       }

       #[test]
       fn test_store_list_elements() {
           let store = InMemoryStore::new();
           for i in 0..3 {
               let elem = ElementData::new(json!({ "index": i }));
               store.create_element(elem);
           }
           let ids = store.list_element_ids();
           assert_eq!(ids.len(), 3);
       }
   }
   ```
   Update `lib.rs`:
   ```rust
   pub mod element;
   pub mod pile;
   pub mod progression;
   pub mod store;
   ```

2. **CLI in `agentic_cli/src/main.rs`**  
   ```rust
   // agentic_cli/src/main.rs
   use clap::{Parser, Subcommand};
   use agentic_core::store::InMemoryStore;
   use agentic_core::element::ElementData;
   use serde_json::Value;

   #[derive(Debug, Parser)]
   #[command(name="lion-cli", version="0.0.1a")]
   struct Cli {
       #[command(subcommand)]
       command: Commands,
   }

   #[derive(Debug, Subcommand)]
   enum Commands {
       CreateElement {
           #[arg(long)]
           metadata: String,
       },
       ListElements,
   }

   fn main() {
       let cli = Cli::parse();
       // Ephemeral store each run (Phase 1 is purely in-memory)
       let store = InMemoryStore::new();

       match cli.command {
           Commands::CreateElement { metadata } => {
               let parsed: Value = serde_json::from_str(&metadata)
                   .expect("Invalid JSON for --metadata");
               let elem = ElementData::new(parsed);
               let id = store.create_element(elem);
               println!("Created element with ID: {id}");
           }
           Commands::ListElements => {
               let ids = store.list_element_ids();
               if ids.is_empty() {
                   println!("No elements stored yet.");
               } else {
                   println!("Element IDs:");
                   for id in ids {
                       println!("{id}");
                   }
               }
           }
       }
   }
   ```

3. **Test the CLI**  
   - `cargo run -p agentic_cli -- create-element --metadata '{"test":"data"}'`
     - Expect output: “Created element with ID: ...”
   - `cargo run -p agentic_cli -- list-elements`
     - Expect either an ID list or “No elements stored yet.” on a fresh store.

### 3.D. **Testing & Validation**

1. **Compile & Lint**  
   ```bash
   cargo fmt --all
   cargo clippy --all-targets
   ```
   Fix any warnings or suggestions.

2. **Run Unit Tests**  
   - `cargo test -p agentic_core`  
   Ensure all pass (for `element.rs`, `pile.rs`, `progression.rs`, `store.rs`).

3. **Manual CLI Test**  
   1. `cargo run -p agentic_cli -- create-element --metadata '{"message":"Hello"}'`  
      - Should print “Created element with ID: ...”  
   2. `cargo run -p agentic_cli -- list-elements`  
      - Should list the ID created in step 1.

4. **Integration Tests** *(Optional at Phase 1)*  
   - You can create a file like `tests/cli_integration.rs` to spawn the CLI commands using `std::process::Command`. Example:
     ```rust
     // tests/cli_integration.rs
     #[test]
     fn test_create_and_list() {
         // 1. create-element
         // 2. list-elements
         // parse output
         // verify the ID is present
     }
     ```
   - This is optional, but recommended for thorough coverage.

5. **Tag & Report**  
   - Once everything is stable, create a Git tag:
     ```bash
     git tag v0.0.1a-phase1
     git push origin v0.0.1a-phase1
     ```
   - Generate a short Phase 1 report (e.g., in `docs/phase1_report.md`) describing:
     - Objectives
     - Implementation summary
     - Tests and validation steps
     - Next steps for Phase 2

**At this point, Phase 1 is complete**. The code now has a **Rust workspace**, **core data structures**, an **in-memory store**, a **CLI** for creating/listing elements, and is thoroughly tested.

---

## 4. **Common Pitfalls & Troubleshooting**

1. **Workspace Not Recognized**  
   - If `cargo build` doesn’t find subcrates, verify `[workspace]` in top-level `Cargo.toml` matches the crate folders.

2. **`serde_json::Value` Parsing Errors**  
   - Ensure the `metadata` string is valid JSON. If you see “Invalid JSON for --metadata”, check the format.

3. **Mutex Poisoning**  
   - If a test panics within a locked section, the mutex is “poisoned.” Usually re-run tests or ensure code doesn’t panic.

4. **Concurrency Deadlock**  
   - Keep your lock usage short. The examples are straightforward enough not to cause deadlocks.

5. **Not Seeing Elements Persist**  
   - Phase 1 store is ephemeral (re-initialized each CLI run). This is expected. We handle persistent or advanced event sourcing in later phases.

---

## 5. **Next Steps (Transition to Phase 2)**

After Phase 1 is complete:

- **Orchestrator & SystemEvents**: In Phase 2, you will introduce a real event loop or an Actix-based orchestrator.  
- **Integration with CLI**: The next CLI commands might involve “submit-task” or “orchestrator-run” subcommands.  
- **Deeper concurrency tests**: Ensure tasks flow properly in your new orchestrator logic.

**Phase 1** is primarily about **foundations**: a stable project structure, concurrency-safe data model, and a minimal but functional CLI. With these pieces validated, you’re ready to tackle advanced orchestration in Phase 2.

---

# Final Summary

**By following these instructions, you’ll have**:
1. A **Rust workspace** with library (`agentic_core`) and CLI (`agentic_cli`).  
2. Solid **core primitives** (`ElementData`, `Pile<T>`, `Progression`) tested for concurrency.  
3. A basic **InMemoryStore** that can create/list elements.  
4. A **CLI** confirming that the store works in ephemeral mode.  
5. A thorough test suite.  
6. Phase 1 code tagged as `v0.0.1a-phase1` and documented in a short phase report.

**Congratulations!** You have completed Phase 1 and laid the foundation for the lion microkernel. Now we move forward to orchestrator logic in Phase 2.