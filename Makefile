.PHONY: version build release help

%:
	@:

version:
	@VERSION="$(filter-out $@,$(MAKECMDGOALS))"; \
	if [ -z "$$VERSION" ]; then \
		echo "Error: Missing version. Ex: make version 1.2.3"; \
		exit 1; \
	fi; \
	awk -v new_ver="$$VERSION" '/^\[package\]/ {p=1} /^version =/ && p {sub(/".*"/, "\"" new_ver "\""); p=0} 1' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml; \
	sed -i 's/"tag_name": "v[^"]*"/"tag_name": "v'"$$VERSION"'"/' .mock/event.json; \
	echo "Updated version: $$VERSION";

build:
	cargo build --release

release:
	@if ! command -v act &> /dev/null; then \
		echo "Error: 'act' is not installed. Please install it to run the release workflow."; \
		exit 1; \
	fi; \
	act release -e .mock/event.json --artifact-server-path ./dist

help:
	@echo "Usage: make <target>"
	@echo
	@echo "Available targets:"
	@echo "  build        Build the project in release mode"
	@echo "  release      Run GitHub Actions release workflow using act"
	@echo "  version      Update project version: make version X.Y.Z"
	@echo "  help         Show this help message"