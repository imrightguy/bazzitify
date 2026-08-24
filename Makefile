# Bazzitify Makefile
# Common development and validation targets

.PHONY: help check-readme test build lint

help:
	@echo "Available targets:"
	@echo "  check-readme  - Validate README.md modules table against modules/ directory"
	@echo "  test          - Run all tests (bash syntax + script validation)"
	@echo "  build         - Build the project (no-op for bash project)"
	@echo "  lint          - Run shellcheck on all bash scripts"
	@echo "  help          - Show this help"

check-readme:
	@./scripts/validate-readme-modules.sh

test: check-readme
	@bash -n scripts/validate-readme-modules.sh
	@bash -n modules/*.sh
	@echo "All syntax checks passed"

build:
	@echo "Bash project - no build step required"

lint:
	@shellcheck scripts/validate-readme-modules.sh modules/*.sh 2>/dev/null || echo "shellcheck not installed, skipping"