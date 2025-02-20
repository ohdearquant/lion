# Contributing to lion

First off, thank you for considering contributing to lion! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code.

## Development Process

1. **Branch Strategy**
   - `main` is the primary branch
   - Create feature branches from `main`
   - Use conventional commit messages
   - Submit PRs back to `main`

2. **Branch Naming**
   - Features: `feature/description`
   - Fixes: `fix/description`
   - Docs: `docs/description`
   - Example: `feature/add-event-sourcing`

3. **Commit Messages**
   Follow conventional commits:
   ```
   type(scope): description

   [optional body]

   [optional footer]
   ```
   Types:
   - feat: New feature
   - fix: Bug fix
   - docs: Documentation only
   - style: Code style changes
   - refactor: Code changes that neither fix bugs nor add features
   - test: Adding or modifying tests
   - chore: Maintenance tasks

4. **Code Style**
   - Run `cargo fmt` before committing
   - Run `cargo clippy` and address all warnings
   - Follow Rust API guidelines
   - Document public APIs
   - Add tests for new functionality

5. **Pull Request Process**
   - Create a PR with a clear title and description
   - Link any related issues
   - Ensure all CI checks pass
   - Request review from maintainers
   - Address review feedback
   - Keep PRs focused and reasonably sized

6. **Testing**
   - Write unit tests for new code
   - Include integration tests where appropriate
   - Ensure all tests pass locally
   - Add documentation tests for public APIs

## Local Development Setup

1. **Prerequisites**
   - Rust (stable channel)
   - Cargo
   - Git

2. **Setup Steps**
   ```bash
   # Clone the repository
   git clone https://github.com/yourusername/lion.git
   cd lion

   # Create a new branch
   git checkout -b feature/your-feature

   # Build the project
   cargo build

   # Run tests
   cargo test
   ```

3. **Pre-commit Checks**
   ```bash
   # Format code
   cargo fmt

   # Run clippy
   cargo clippy --all-targets -- -D warnings

   # Run tests
   cargo test
   ```

## Documentation

1. **Code Documentation**
   - Document all public APIs
   - Include examples in doc comments
   - Keep documentation up to date with changes

2. **Project Documentation**
   - Update README.md for significant changes
   - Add/update docs/ for new features
   - Include architecture decisions in docs/

## Issue Reporting

1. **Bug Reports**
   - Use the bug report template
   - Include steps to reproduce
   - Provide system information
   - Include relevant logs

2. **Feature Requests**
   - Use the feature request template
   - Explain the use case
   - Describe expected behavior
   - Provide examples if possible

## Review Process

1. **Code Review Guidelines**
   - Check code style and formatting
   - Verify test coverage
   - Review documentation
   - Check for security implications
   - Ensure backward compatibility

2. **PR Acceptance Criteria**
   - Passes all CI checks
   - Has appropriate test coverage
   - Includes documentation
   - Follows code style guidelines
   - Addresses review comments

## Getting Help

- Open an issue for questions
- Join our community discussions
- Read the documentation
- Contact maintainers

Thank you for contributing to lion!