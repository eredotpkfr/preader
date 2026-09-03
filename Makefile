SHELL=/bin/bash

.PHONY: all

all: \
	book-build \
	book-test \
	cargo-build \
	cargo-check \
	cargo-clean \
	cargo-doc \
	cargo-doc-rs \
	cargo-doc-test \
	cargo-fix \
	cargo-machete \
	cargo-nextest \
	cargo-test \
	cargo-udeps \
	clippy \
	coverage \
	coverage-lcov \
	deny \
	develop \
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
	install-pre-commit-hooks \
	install-pre-commit-linux \
	install-pre-commit-mac \
	install-uv-linux \
	install-uv-mac \
	live-book \
	maturin-generate-ci \
	mypy \
	pytest \
	ruff-check \
	ruff-check-fix \
	ruff-format \
	ruff-format-check \
	rustfmt \
	rustfmt-check \
	rustup \
	shell \
	stubs \
	stubtest \
	stubtest-allowlist \
	update-pre-commit-hooks \
	uv-audit \
	uv-create-venv

book-build:
	@mdbook build book
book-test:
	@mdbook test book
cargo-build:
	@uv run cargo build
cargo-check:
	@uv run cargo check
cargo-clean:
	@cargo clean
cargo-doc:
	@uv run cargo doc
cargo-doc-rs:
	@uv run cargo +nightly docs-rs
cargo-doc-test:
	@uv run cargo test --doc
cargo-fix:
	@uv run cargo fix --allow-dirty --allow-staged
cargo-machete:
	@cargo machete
cargo-nextest:
	@uv run cargo nextest run
cargo-test:
	@uv run cargo test
cargo-udeps:
	@uv run cargo +nightly udeps --all-targets
clippy:
	@uv run cargo clippy --all-targets --all-features -- -D warnings
coverage: COVERAGE_REPORT := --html --open
coverage-lcov: COVERAGE_REPORT := --lcov --output-path lcov.info
coverage coverage-lcov:
	@( eval "$$(cargo llvm-cov show-env --sh)" && \
		cargo llvm-cov clean --workspace && \
		$(MAKE) --no-print-directory cargo-test cargo-doc-test pytest && \
		cargo llvm-cov report $(COVERAGE_REPORT) ); \
	status=$$?; cargo clean -p preader >/dev/null; \
		$(MAKE) --no-print-directory develop; exit $$status
deny:
	@cargo deny --all-features --log-level error check
develop:
	@uv run python -c "import pathlib, shutil, sysconfig; shutil.rmtree(pathlib.Path(sysconfig.get_paths()['purelib']) / 'preader', ignore_errors=True)"
	@uv run maturin develop --uv
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
install-pre-commit-hooks:
	@pre-commit install --install-hooks
	@pre-commit install --hook-type commit-msg --install-hooks
install-pre-commit-linux:
	@sudo apt install pre-commit
install-pre-commit-mac:
	@brew install pre-commit
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
rustfmt: cargo-fix
	@cargo +nightly fmt --all
rustfmt-check:
	@cargo +nightly fmt --all -- --check
rustup:
	@rustup self update
	@rustup update
shell: develop
	@uv run python3
stubs:
	@uv run maturin generate-stubs -q --out python -F experimental-inspect
stubtest: develop
	@uv run stubtest preader --concise --allowlist stubtest-allowlist.txt
stubtest-allowlist: develop
	@uv run stubtest preader --generate-allowlist > stubtest-allowlist.txt
update-pre-commit-hooks:
	@pre-commit autoupdate
uv-audit:
	@uv audit
uv-create-venv:
	@uv venv --python $(shell python3 --version | cut -d" " -f2)
