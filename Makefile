.PHONY: version build release help

version:
	@if [ -z "$(filter-out $@,$(MAKECMDGOALS))" ]; then \
		echo "Usage: make version X.Y.Z"; exit 1; \
	fi; \
	VERSION="$(filter-out $@,$(MAKECMDGOALS))"; \
	echo "Setting version to $$VERSION"; \
	# Update Cargo.toml version field (first match)
	sed -E -i.bak '0,/^version = .*/s//version = "'"$$VERSION"'"/' Cargo.toml; \
	# Update .mock/event.json tag_name (add leading v if missing in file)
	if [ -f .mock/event.json ]; then \
		sed -E -i.bak 's/("tag_name"\s*:\s*")v?[0-9]+\.[0-9]+\.[0-9]+("?)/\1v'"$$VERSION"'\2/' .mock/event.json; \
	fi; \
	echo "Updated Cargo.toml and .mock/event.json"

build:
	cargo build --release

release:
	@if ! command -v act &> /dev/null; then \
		echo "Error: 'act' is not installed. Please install it to run the release workflow."; \
		exit 1; \
	fi
	act release -e .mock/event.json --artifact-server-path ./dist

help:
	@echo "Usage: make <target>"
	@echo
	@echo "Available targets:"
	@echo "  build        Build the project in release mode"
	@echo "  release      Run GitHub Actions release workflow using act"
	@echo "  version      Update project version: make version X.Y.Z"
	@echo "  help         Show this help message"