# Contributing to Sakko

Thanks for your interest in contributing!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/Sakko.git`
3. Install Rust via [rustup](https://rustup.rs) (the pinned toolchain in `rust-toolchain.toml` is used automatically)
4. Create a feature branch: `git checkout -b feat/your-feature`

## Development

```bash
cargo build      # build the workspace
cargo test       # run tests
cargo clippy     # lint
cargo fmt        # format
```

## Pull Requests

- Keep changes focused. One feature or fix per PR.
- Add tests for new functionality
- Ensure all checks pass (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`)
- Update CHANGELOG.md if applicable

## Code Style

- `rustfmt` and `clippy` are the source of truth and CI enforces both with warnings as errors.
- Follow the existing patterns in the codebase
- Avoid adding comments unless necessary for clarity

## Reporting Issues

Use the GitHub issue tracker with the appropriate template.
