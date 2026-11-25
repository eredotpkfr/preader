SHELL=/bin/bash

BOLD := $(shell tput bold 2>/dev/null)
CYAN := $(shell tput setaf 6 2>/dev/null)
DIM := $(shell tput dim 2>/dev/null)
GREEN := $(shell tput setaf 2 2>/dev/null)
RED := $(shell tput setaf 1 2>/dev/null)
RESET := $(shell tput sgr0 2>/dev/null)

TARGETS := \
	book-build \
	book-test \
	cargo-build \
	cargo-check \
	cargo-clean \
	cargo-clippy \
	cargo-deny \
	cargo-doc \
	cargo-doc-rs \
	cargo-doc-test \
	cargo-fix \
	cargo-machete \
	cargo-nextest \
	cargo-rustfmt \
	cargo-rustfmt-check \
	cargo-test \
	cargo-udeps \
	cargo-update \
	coverage \
	coverage-codecov \
	coverage-lcov \
	develop \
	examples-python \
	examples-rust \
	install-cargo-clippy \
	install-cargo-deny \
	install-cargo-doc-rs \
	install-cargo-llvm-cov \
	install-cargo-machete \
	install-cargo-mdbook \
	install-cargo-nextest \
	install-cargo-tools \
	install-cargo-udeps \
	install-nightly-toolchain \
	install-pre-commit \
	install-pre-commit-hooks \
	install-uv-linux \
	install-uv-mac \
	live-book \
	maturin-generate-ci \
	mypy \
	pre-commit \
	pre-commit-update-hooks \
	pytest \
	ruff-check \
	ruff-check-fix \
	ruff-format \
	ruff-format-check \
	rustup \
	shell \
	stubs \
	stubtest \
	stubtest-allowlist \
	uv-audit \
	uv-create-venv

ifneq ($(filter examples-python examples-rust,$(firstword $(MAKECMDGOALS))),)
EXAMPLE ?= $(filter-out $(TARGETS),$(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS)))
$(eval $(EXAMPLE):;@:)
endif
EXAMPLES_PYTHON := $(or $(EXAMPLE),$(sort $(basename $(notdir $(wildcard examples/python/*.py)))))
EXAMPLES_RUST := $(or $(EXAMPLE),$(sort $(basename $(notdir $(wildcard examples/rust/*.rs)))))

define run-examples
	@total=$(words $(3)); index=0; \
	for example in $(3); do \
		index=$$((index + 1)); \
		printf '\n$(BOLD)$(CYAN)▶ [%d/%d] %s$(RESET) $(DIM)examples/$(1)/%s.$(2)$(RESET)\n\n' \
			"$$index" "$$total" "$$example" "$$example"; \
		$(4) || { printf '\n$(BOLD)$(RED)✗ %s failed$(RESET)\n' "$$example"; exit 1; }; \
	done; \
	printf '\n$(BOLD)$(GREEN)✓ %d $(1) examples passed$(RESET)\n' "$$total"
endef

.DEFAULT_GOAL := help
.PHONY: help $(TARGETS)

help:
	@echo "$(BOLD)$(CYAN)Usage:$(RESET) make $(BOLD)<target>$(RESET), where $(BOLD)<target>$(RESET) is one of:"
	@echo "$(TARGETS)" | sed 's/ /, /g' | fold -s -w 76 | sed 's/^/  /; s/ *$$//'

book-build:
	@mdbook build book
book-test:
	@mdbook test book
cargo-build:
	@uv run cargo build --features python,experimental-inspect
cargo-check:
	@uv run cargo check --all-features --all-targets
cargo-clean:
	@cargo clean
cargo-clippy:
	@uv run cargo clippy --all-targets --all-features -- -D warnings
cargo-deny:
	@cargo deny --all-features --log-level error check
cargo-doc:
	@uv run cargo doc --all-features
cargo-doc-rs:
	@cargo +nightly docs-rs
cargo-doc-test:
	@cargo test --doc --features testing
cargo-fix:
	@uv run cargo fix --all-features --allow-dirty --allow-staged
cargo-machete:
	@cargo machete
cargo-nextest:
	@cargo nextest run --features testing --config-file nextest.toml
cargo-rustfmt: cargo-fix
	@cargo +nightly fmt --all
cargo-rustfmt-check:
	@cargo +nightly fmt --all -- --check
cargo-test:
	@cargo test --features testing
cargo-udeps:
	@uv run cargo +nightly udeps --all-targets --all-features
cargo-update:
	@cargo update --verbose
coverage: COVERAGE_REPORT := --html --open
coverage-codecov: COVERAGE_REPORT := --codecov --output-path codecov.json
coverage-lcov: COVERAGE_REPORT := --lcov --output-path lcov.info
coverage coverage-codecov coverage-lcov:
	@( eval "$$(cargo llvm-cov show-env --sh)" && \
		cargo llvm-cov clean --workspace && \
		$(MAKE) --no-print-directory cargo-test cargo-doc-test pytest && \
		cargo llvm-cov report $(COVERAGE_REPORT) ); \
	status=$$?; cargo clean -p preader >/dev/null; \
		$(MAKE) --no-print-directory develop; exit $$status
develop:
	@uv run python -c "import pathlib, shutil, sysconfig; shutil.rmtree(pathlib.Path(sysconfig.get_paths()['purelib']) / 'preader', ignore_errors=True)"
	@uv run maturin develop --uv
examples-python: develop
	$(call run-examples,python,py,$(EXAMPLES_PYTHON),uv run --no-sync python examples/python/$$example.py)
examples-rust:
	$(call run-examples,rust,rs,$(EXAMPLES_RUST),cargo run --quiet --example $$example)
install-cargo-clippy:
	@rustup component add clippy
install-cargo-deny:
	@cargo install cargo-deny --locked
install-cargo-doc-rs:
	@cargo install cargo-docs-rs
install-cargo-llvm-cov:
	@cargo install cargo-llvm-cov --locked
install-cargo-machete:
	@cargo install cargo-machete --locked
install-cargo-mdbook:
	@cargo install mdbook
install-cargo-nextest:
	@cargo install cargo-nextest --locked
install-cargo-tools: \
	install-cargo-clippy \
	install-cargo-deny \
	install-cargo-doc-rs \
	install-cargo-llvm-cov \
	install-cargo-machete \
	install-cargo-nextest \
	install-cargo-udeps \
	install-nightly-toolchain
install-cargo-udeps:
	@cargo install cargo-udeps --locked
install-nightly-toolchain:
	@rustup toolchain install nightly
install-pre-commit:
	@uv tool install pre-commit
install-pre-commit-hooks:
	@pre-commit install --install-hooks
	@pre-commit install --hook-type commit-msg --install-hooks
install-uv-linux:
	@curl -LsSf https://astral.sh/uv/install.sh | sh
install-uv-mac:
	@brew install uv
live-book: book-test
	@mdbook serve book
maturin-generate-ci:
	@uv run maturin generate-ci github
mypy:
	@uv run mypy
pre-commit:
	@pre-commit run -a
pre-commit-update-hooks:
	@pre-commit autoupdate
pytest: develop
	@uv run pytest
ruff-check:
	@uv run ruff check
ruff-check-fix:
	@uv run ruff check --fix
ruff-format:
	@uv run ruff format
ruff-format-check:
	@uv run ruff format --check
rustup:
	@rustup self update
	@rustup update
shell: develop
	@uv run python
stubs:
	@uv run maturin generate-stubs -q --out python -F extension-module,experimental-inspect
stubtest: develop
	@uv run stubtest preader --concise --allowlist stubtest-allowlist.txt
stubtest-allowlist: develop
	@uv run stubtest preader --generate-allowlist > stubtest-allowlist.txt
uv-audit:
	@uv audit
uv-create-venv:
	@uv venv --allow-existing $(if $(PYTHON),--python $(PYTHON))
