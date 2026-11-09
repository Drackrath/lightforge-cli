# Local helpers. Everyday work is plain `cargo build` / `cargo test`; release
# binaries are built and published by `.github/workflows/release.yml`.

# For `make install` only; the packages set their own roots.
PREFIX ?= /usr/local
DESTDIR ?=

BIN := target/release/lightforge

# The highest glibc a release may ask for, and what the deb and rpm declare.
# Ubuntu 22.04 is 2.35, and the release runners are Ubuntu 22.04.
GLIBC_MAX ?= 2.35

VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

.PHONY: build install uninstall cli-doc check-cli-doc check-glibc check-spec

build:
	cargo build --release --locked -p lightforge-cli

## Install a source build the way a package would.
## `PREFIX=$$HOME/.local make install` needs no root.
install:
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/lightforge
	install -Dm644 LICENSE $(DESTDIR)$(PREFIX)/share/licenses/lightforge/LICENSE
	@echo "installed to $(DESTDIR)$(PREFIX)"

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/lightforge
	rm -rf $(DESTDIR)$(PREFIX)/share/licenses/lightforge
	@echo "removed from $(DESTDIR)$(PREFIX)"

## Regenerate docs/CLI.md from the binary's own --help.
cli-doc: build
	sh scripts/gen-cli.sh

## Fail if docs/CLI.md is not what the current binary would produce.
check-cli-doc: cli-doc
	@git diff --quiet -- docs/CLI.md \
	  || { echo "docs/CLI.md is out of date — commit the regenerated page"; exit 1; }
	@echo "ok   docs/CLI.md matches the binary"

## Fail if the binary demands a glibc newer than the floor.
check-glibc:
	@[ -f "$(BIN)" ] || { echo "missing $(BIN) — run \`make build\` first"; exit 1; }
	@needs=$$(objdump -T "$(BIN)" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1); \
	highest=$${needs#GLIBC_}; \
	top=$$(printf '%s\n%s\n' "$(GLIBC_MAX)" "$$highest" | sort -V | tail -1); \
	if [ "$$top" = "$(GLIBC_MAX)" ]; then \
	  echo "ok   $(BIN) needs glibc $$highest (floor $(GLIBC_MAX))"; \
	else \
	  echo "FAIL $(BIN) needs glibc $$highest, above the $(GLIBC_MAX) floor"; exit 1; \
	fi

## The COPR spec carries its version by hand, because COPR builds it from git
## and never sees this Makefile.
check-spec:
	@got=$$(sed -n 's/^%global upstream_version //p' packaging/rpm/lightforge.spec); \
	[ "$$got" = "$(VERSION)" ] || { echo "packaging/rpm/lightforge.spec is $$got, Cargo.toml is $(VERSION)"; exit 1; }
	@echo "ok   spec is $(VERSION)"
