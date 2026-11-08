# makefile
.DEFAULT_GOAL := help

.PHONY: help
help:  # show this list
	@echo "---- list of available commands ---"
	@grep -E '^[[:alnum:]_/-]+ *:.*?#.*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?# "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: run run-front
run:  # run the Tauri desktop application
	@npm run tauri -- dev
run-front:  # run the React frontend only (bind to 0.0.0.0)
	@npm run dev -- --host 0.0.0.0

.PHONY: build build-front
build:  # build the desktop application bundle
	@npm run tauri -- build
build-front:  # build the React frontend only
	@npm run build

.PHONY: test test-rust check
test:  # run all available tests and frontend type checks
	@npm run build
	@cd src-tauri && cargo test
test-rust:  # run Rust tests only
	@cd src-tauri && cargo test
check:  # run formatting checks and all tests
	@cd src-tauri && cargo fmt --all -- --check
	@$(MAKE) test

.PHONY: fmt
fmt:  # format Rust sources
	@cd src-tauri && cargo fmt --all

.PHONY: gen-icons
gen-icons:  # generate desktop icons from src-tauri/icons/app.svg
	@npm run tauri -- icon src-tauri/icons/app.svg
	@rm -rf src-tauri/icons/android src-tauri/icons/ios
	@rm -f src-tauri/icons/Square*.png src-tauri/icons/StoreLogo.png
