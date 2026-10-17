# Mesh - Makefile wrapper forwarding to just (https://github.com/casey/just)
# All canonical tasks are defined in justfile.

.PHONY: default
default:
	@just

ifeq (version,$(firstword $(MAKECMDGOALS)))
  VERSION_ARG := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(VERSION_ARG):;@:)
endif

ifeq (release,$(firstword $(MAKECMDGOALS)))
  RELEASE_ARG := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(RELEASE_ARG):;@:)
endif

ifeq (dev,$(firstword $(MAKECMDGOALS)))
  DEV_ARG := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(DEV_ARG):;@:)
endif

version:
	@just version $(VERSION_ARG)

release:
	@just release $(RELEASE_ARG)

dev:
	@just dev $(DEV_ARG)

%:
	@just "$@"
