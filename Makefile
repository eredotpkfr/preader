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
	pytest \
	rustfmt \
	rustfmt-check \
	rustup \
	shell \
	update-pre-commit-hooks \
	uv-create-venv

book-build:
	@uv run mdbook build book
book-test:
	@uv run mdbook test book
cargo-build:
	@uv run cargo build
cargo-check:
	@uv run cargo check
cargo-clean:
	@uv run cargo clean
cargo-doc:
	@uv run cargo doc
cargo-doc-rs:
	@uv run cargo +nightly docs-rs
cargo-doc-test:
	@uv run cargo test --doc
cargo-fix:
	@uv run cargo fix --allow-dirty --allow-staged
cargo-machete:
	@uv run cargo machete
cargo-nextest:
	@uv run cargo nextest run
cargo-test:
	@uv run cargo test
cargo-udeps:
	@uv run cargo +nightly udeps
clippy:
	@uv run cargo clippy --all-targets --all-features
coverage:
	@uv run cargo +nightly llvm-cov \
		--all-features \
		--workspace \
		--doctests \
		--html \
		--open
deny:
	@uv run cargo deny --all-features --log-level error check
develop:
	@uv run python -c "import pathlib, shutil, sysconfig; shutil.rmtree(pathlib.Path(sysconfig.get_paths()['purelib']) / 'preader', ignore_errors=True)"
	@uv run maturin develop --uv
install-cargo-clippy:
	@uv run rustup component add clippy
install-cargo-deny:
	@uv run cargo install cargo-deny --locked
install-cargo-doc-rs:
	@uv run cargo install cargo-docs-rs
install-cargo-llvm-cov:
	@uv run cargo install cargo-llvm-cov --locked
install-cargo-machete:
	@uv run cargo install cargo-machete --locked
install-cargo-mdbook:
	@uv run cargo install mdbook
install-cargo-nextest:
	@uv run cargo install cargo-nextest --locked
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
	@uv run cargo install cargo-udeps --locked
install-nightly-toolchain:
	@uv run rustup toolchain install nightly
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
	@uv run mdbook serve book
maturin-generate-ci:
	@uv run maturin generate-ci github
pytest: develop
	@uv run pytest
rustfmt: cargo-fix
	@uv run cargo +nightly fmt --all
rustfmt-check:
	@uv run cargo +nightly fmt --all -- --check
rustup:
	@rustup self update
	@rustup update
shell: develop
	@uv run python3
update-pre-commit-hooks:
	@pre-commit autoupdate
uv-create-venv:
	@uv venv --python $(shell python3 --version | cut -d" " -f2)
