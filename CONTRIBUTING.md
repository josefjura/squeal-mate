# Contributing to SquealMate

SquealMate is a small, solo-maintained project. Contributions are welcome, but please
keep expectations modest around response time — reviews and releases happen when time allows.

## Building and testing

```bash
cargo build           # debug build
cargo test             # run the test suite
cargo run -- migrations  # run the app locally
```

## Before opening a PR

Run these locally and make sure they pass — CI enforces both:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

Keep changes focused. This project favors simplicity and reliability over
abstraction, so prefer the smallest change that solves the problem.

## Reporting bugs

Please use the bug report issue template and include:

- SquealMate version (`squealmate --version`)
- Operating system
- SQL Server version (if relevant)
- Steps to reproduce, and what you expected to happen instead

## Pull requests

Use the PR template checklist. A short description of *why* the change is
needed is more useful than a description of *what* changed — the diff already
shows that.
