## Description
This PR implements Phase 1 of the Liongate project, establishing the core primitives, CLI interface, and project infrastructure. It sets up the foundation for our event-driven orchestration system with a microkernel architecture.

## Type of Change
- [x] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [x] This change requires a documentation update

## How Has This Been Tested?
- [x] Unit Tests
  - Core primitives (ElementData, Pile, Progression)
  - Store operations
  - Concurrency safety tests
- [x] Integration Tests
  - CLI functionality
  - Store integration
- [x] Manual Testing
  - CLI element creation and listing
  - Formatting and linting checks
  - CI pipeline verification

## Test Configuration
- Rust version: stable (rustc 1.75.0)
- OS: Ubuntu Latest (CI), Local development machines

## Checklist
- [x] My code follows the style guidelines of this project
- [x] I have performed a self-review of my own code
- [x] I have commented my code, particularly in hard-to-understand areas
- [x] I have made corresponding changes to the documentation
- [x] My changes generate no new warnings
- [x] I have added tests that prove my fix is effective or that my feature works
- [x] New and existing unit tests pass locally with my changes
- [x] Any dependent changes have been merged and published in downstream modules

## Additional Notes
This PR sets up:
1. Project Structure
   - Workspace with two crates
   - Core primitives implementation
   - Basic CLI interface

2. Development Infrastructure
   - GitHub Actions CI pipeline
   - Formatting and linting standards
   - Issue and PR templates
   - Documentation guidelines

3. Documentation
   - README.md with project overview
   - CONTRIBUTING.md with guidelines
   - CHANGELOG.md for version tracking
   - Code-level documentation

Next steps will involve Phase 2: implementing the microkernel orchestrator and system events.