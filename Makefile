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
	if [ -f .mock/event.json ]; then \
		sed -i 's/"tag_name": "v[^"]*"/"tag_name": "v'"$$VERSION"'"/' .mock/event.json; \
	fi; \
	echo "Updated version: $$VERSION";

build:
	cargo build --release

release:
	@if ! command -v act &> /dev/null; then \
		echo "Error: 'act' is not installed. Please install it to run the release workflow locally."; \
		exit 1; \
	fi; \
	TARGET="$(filter-out $@,$(MAKECMDGOALS))"; \
	mkdir -p ./dist; \
	if [ -z "$$TARGET" ]; then \
		echo "Running Linux and Windows release workflows locally via act..."; \
		act release -e .mock/event.json --artifact-server-path ./dist \
			-W .github/workflows/release-linux.yml \
			-W .github/workflows/release-windows.yml \
			-P windows-latest=catthehacker/ubuntu:act-latest \
			-P ubuntu-24.04-arm=catthehacker/ubuntu:act-latest; \
	elif [ "$$TARGET" = "linux" ]; then \
		echo "Running Linux release workflow..."; \
		act release -e .mock/event.json --artifact-server-path ./dist \
			-W .github/workflows/release-linux.yml \
			-P ubuntu-24.04-arm=catthehacker/ubuntu:act-latest; \
	elif [ "$$TARGET" = "windows" ]; then \
		echo "Running Windows release workflow..."; \
		act release -e .mock/event.json --artifact-server-path ./dist \
			-W .github/workflows/release-windows.yml \
			-P windows-latest=catthehacker/ubuntu:act-latest; \
	elif [ "$$TARGET" = "mac" ] || [ "$$TARGET" = "macos" ]; then \
		echo "Note: act on Linux cannot emulate macOS runners (no macOS Docker images)."; \
		echo "Running macOS release workflow test if macos runner is available..."; \
		act release -e .mock/event.json --artifact-server-path ./dist \
			-W .github/workflows/release-macos.yml || true; \
	else \
		echo "Error: Unknown target '$$TARGET'. Use 'linux', 'windows', 'mac', or leave empty for all."; \
		exit 1; \
	fi

help:
	@echo "Usage: make <target>"
	@echo
	@echo "Available targets:"
	@echo "  build              Build the project in release mode for host system"
	@echo "  release            Run release workflows using act (local testing)"
	@echo "  release linux      Run only the Linux release workflow locally"
	@echo "  release windows    Run only the Windows release workflow locally"
	@echo "  release mac        Run/test macOS release workflow (requires macOS or remote runner)"
	@echo "  version            Update project version: make version X.Y.Z"
	@echo "  help               Show this help message"