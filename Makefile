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
	TARGET="$(filter-out $@,$(MAKECMDGOALS))"; \
	if [ -z "$$TARGET" ]; then \
		echo "Running all release workflows..."; \
		act release -e .mock/event.json --artifact-server-path ./dist -W .github/workflows/release-linux.yml -W .github/workflows/release-windows.yml -P windows-latest=catthehacker/ubuntu:act-latest; \
	elif [ "$$TARGET" = "linux" ]; then \
		echo "Running Linux release workflow..."; \
		act release -e .mock/event.json --artifact-server-path ./dist -W .github/workflows/release-linux.yml; \
	elif [ "$$TARGET" = "windows" ]; then \
		echo "Running Windows release workflow..."; \
		act release -e .mock/event.json --artifact-server-path ./dist -W .github/workflows/release-windows.yml -P windows-latest=catthehacker/ubuntu:act-latest; \
	else \
		echo "Error: Unknown target '$$TARGET'. Use 'linux', 'windows', or leave empty for all."; \
		exit 1; \
	fi

help:
	@echo "Usage: make <target>"
	@echo
	@echo "Available targets:"
	@echo "  build              Build the project in release mode"
	@echo "  release            Run all GitHub Actions release workflows using act"
	@echo "  release linux      Run only the Linux release workflow"
	@echo "  release windows    Run only the Windows release workflow"
	@echo "  version            Update project version: make version X.Y.Z"
	@echo "  help               Show this help message"