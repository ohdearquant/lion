# LionForge IDE Commit Message Guide

Follow the
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
specification. This helps automate changelog generation and makes commit history
more readable.

## Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

## 1. Type

Must be one of the following:

- **feat:** A new feature (correlates with MINOR in SemVer).
- **fix:** A bug fix (correlates with PATCH in SemVer).
- **build:** Changes that affect the build system or external dependencies
  (e.g., `Cargo.toml`, `package.json`, Dockerfile).
- **chore:** Other changes that don't modify source or test files (e.g.,
  updating dependencies, configuring linters).
- **ci:** Changes to CI configuration files and scripts.
- **docs:** Documentation only changes.
- **perf:** A code change that improves performance.
- **refactor:** A code change that neither fixes a bug nor adds a feature.
- **revert:** Reverts a previous commit.
- **style:** Changes that do not affect the meaning of the code (white-space,
  formatting, missing semi-colons, etc).
- **test:** Adding missing tests or correcting existing tests.

## 2. Scope (Optional)

The scope provides additional contextual information and is contained within
parentheses. It should be a noun describing the section of the codebase
affected.

Examples:

- `feat(ui): ...`
- `fix(runtime): ...`
- `test(proxy): ...`
- `refactor(workflow-editor): ...`
- `docs(dev-guide): ...`

If the change affects multiple scopes, you can omit it or choose the primary
one.

## 3. Subject

The subject contains a succinct description of the change:

- Use the imperative, present tense: "change" not "changed" nor "changes".
- Don't capitalize the first letter.
- No dot (.) at the end.
- Maximum 50 characters recommended.

Example: `feat(ui): add agent status table component` Example:
`fix(backend): correctly handle runtime shutdown signal`

## 4. Body (Optional)

- Use the imperative, present tense.
- Should include the motivation for the change and contrast this with previous
  behavior.
- Use line breaks (`\n`) for readability.

## 5. Footer(s) (Optional)

- **Breaking Changes:** Start with `BREAKING CHANGE:` followed by a description
  of the change, justification, and migration notes. A `!` after the type/scope
  also indicates a BREAKING CHANGE (e.g., `refactor(core)!: ...`).
- **Referencing Issues:** Use keywords like `Refs: #123`, `Closes: #123`,
  `Fixes: #123`.

## Examples

**Commit with scope, body, and breaking change footer:**

```
feat(api)!: change agent ID format from integer to UUID string

BREAKING CHANGE: Agent IDs are now represented as UUID strings instead of integers.
Requires updating API clients and any stored references.
Affects `list_agents` and `get_agent_details` commands.
```

**Simple fix:**

```
fix(ui): prevent log viewer crash on empty message
```

**Adding tests:**

```
test(backend): add unit tests for grant_capability command
```

**Refactor:**

```
refactor(runtime): extract plugin loading logic into separate module
```

Adhering to this format ensures a clean, navigable, and automatable commit
history.

```
---

This setup provides:

1.  A detailed guide tailored to LionForge, Tauri, and Rust.
2.  Streamlined LLM roles focused on the project's needs.
3.  Prompts emphasizing automated testing and clear outputs.
4.  Essential guidance documents (`CODING_STYLE_*`, `TESTING_STRATEGY`, `DESIGN_DOC_TEMPLATE`, `COMMIT_GUIDE`).

The Orchestrator can now use these updated prompts and the `DEV_GUIDE.md` to drive the development through the defined phases, ensuring the LLM team operates effectively within the specified constraints and quality expectations.
```
