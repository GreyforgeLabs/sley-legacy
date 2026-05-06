.PHONY: fmt test smoke native
SLEY_BIN ?= sley
NATIVE_SOURCES := $(wildcard src/*.sley)

fmt:
	npm run fmt

test:
	npm test

smoke:
	npm run smoke

native:
ifeq ($(NATIVE_SOURCES),)
	@echo "No native Sley sources found in src/"
else
	@if command -v "$(SLEY_BIN)" >/dev/null 2>&1; then \
		for source in $(NATIVE_SOURCES); do \
			"$(SLEY_BIN)" parse "$$source"; \
		done; \
	else \
		echo "SKIP: sley compiler not found (set SLEY_BIN to run Sley parse checks)"; \
	fi
endif
