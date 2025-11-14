# Makefile for SquealMate development workflows

.PHONY: help precommit fmt clippy test build clean review-snapshots

# Default target - show help
help:
	@echo "SquealMate Development Commands:"
	@echo ""
	@echo "  make precommit       - Run all checks before committing (fmt, clippy, build, test, snapshot review)"
	@echo "  make fmt             - Format code with cargo fmt"
	@echo "  make clippy          - Run clippy linter"
	@echo "  make build           - Build the project"
	@echo "  make test            - Run all tests"
	@echo "  make review-snapshots - Review snapshot test changes"
	@echo "  make clean           - Clean build artifacts"
	@echo ""

# Main precommit check - runs everything needed before pushing
precommit: fmt clippy build test review-snapshots
	@echo ""
	@echo "✅ All precommit checks passed!"
	@echo ""
	@echo "If snapshots need review, use 'cargo insta accept' to approve them."
	@echo "Then commit and push your changes."

# Format code
fmt:
	@echo "🎨 Running cargo fmt..."
	@cargo fmt --all -- --check || (echo "❌ Code formatting issues found. Run 'cargo fmt' to fix." && exit 1)
	@echo "✅ Code formatting OK"

# Run clippy (warnings allowed for now, but errors will fail)
clippy:
	@echo "🔍 Running clippy..."
	@cargo clippy --all-targets --all-features
	@echo "✅ Clippy checks passed"

# Build project
build:
	@echo "🔨 Building project..."
	@cargo build --verbose
	@echo "✅ Build successful"

# Run tests
test:
	@echo "🧪 Running tests..."
	@cargo test --verbose
	@echo "✅ All tests passed"

# Review snapshot changes (if any)
review-snapshots:
	@echo "📸 Checking for snapshot changes..."
	@if [ -n "$$(find tests/snapshots -name '*.snap.new' 2>/dev/null)" ]; then \
		echo "⚠️  Snapshot changes detected!"; \
		echo ""; \
		echo "Run 'cargo insta review' to review changes."; \
		echo "Run 'cargo insta accept' to accept all changes."; \
		echo "Run 'cargo insta reject' to reject all changes."; \
		echo ""; \
		cargo insta test --review; \
	else \
		echo "✅ No snapshot changes"; \
	fi

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete"
