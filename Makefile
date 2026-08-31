.PHONY: fmt clippy test precommit

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --verbose

precommit: fmt clippy test
