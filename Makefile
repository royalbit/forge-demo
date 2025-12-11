# Makefile for forge-e2e
# Build multi-platform binaries and publish to GitHub releases

BINARY := forge-e2e
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
REPO := royalbit/forge-demo

# Platforms
PLATFORMS := \
	aarch64-apple-darwin \
	x86_64-apple-darwin \
	x86_64-unknown-linux-gnu \
	aarch64-unknown-linux-gnu \
	x86_64-pc-windows-msvc

# Detect current platform
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),Darwin)
	ifeq ($(UNAME_M),arm64)
		CURRENT_PLATFORM := aarch64-apple-darwin
	else
		CURRENT_PLATFORM := x86_64-apple-darwin
	endif
else ifeq ($(UNAME_S),Linux)
	ifeq ($(UNAME_M),aarch64)
		CURRENT_PLATFORM := aarch64-unknown-linux-gnu
	else
		CURRENT_PLATFORM := x86_64-unknown-linux-gnu
	endif
else
	CURRENT_PLATFORM := x86_64-pc-windows-msvc
endif

# Directories
DIST_DIR := dist
TARGET_DIR := target

.PHONY: all build build-all clean dist publish help

help:
	@echo "forge-e2e build system"
	@echo ""
	@echo "Targets:"
	@echo "  build        Build for current platform ($(CURRENT_PLATFORM))"
	@echo "  build-all    Build for all platforms (requires cross)"
	@echo "  dist         Create release archives in dist/"
	@echo "  publish      Publish to GitHub releases (requires gh CLI)"
	@echo "  clean        Remove build artifacts"
	@echo ""
	@echo "Current version: $(VERSION)"

# Build for current platform
build:
	cargo build --release
	@mkdir -p bin
	@cp $(TARGET_DIR)/release/$(BINARY) bin/

# Build for specific platform (usage: make build-target TARGET=aarch64-apple-darwin)
build-target:
ifndef TARGET
	$(error TARGET is required. Usage: make build-target TARGET=aarch64-apple-darwin)
endif
	@echo "Building for $(TARGET)..."
ifeq ($(TARGET),$(CURRENT_PLATFORM))
	cargo build --release --target $(TARGET)
else
	cross build --release --target $(TARGET)
endif
	@mkdir -p $(DIST_DIR)
ifeq ($(findstring windows,$(TARGET)),windows)
	@cp $(TARGET_DIR)/$(TARGET)/release/$(BINARY).exe $(DIST_DIR)/
else
	@cp $(TARGET_DIR)/$(TARGET)/release/$(BINARY) $(DIST_DIR)/
endif

# Build all platforms (requires cross: cargo install cross)
build-all:
	@echo "Building for all platforms..."
	@mkdir -p $(DIST_DIR)
	@for platform in $(PLATFORMS); do \
		echo ""; \
		echo "=== Building $$platform ==="; \
		if [ "$$platform" = "$(CURRENT_PLATFORM)" ]; then \
			cargo build --release --target $$platform || exit 1; \
		else \
			cross build --release --target $$platform || exit 1; \
		fi; \
	done
	@echo ""
	@echo "All platforms built successfully"

# Create distribution archives
dist: build-all
	@echo "Creating distribution archives..."
	@mkdir -p $(DIST_DIR)
	@for platform in $(PLATFORMS); do \
		echo "Packaging $$platform..."; \
		if echo "$$platform" | grep -q "windows"; then \
			cp $(TARGET_DIR)/$$platform/release/$(BINARY).exe $(DIST_DIR)/$(BINARY).exe; \
			cd $(DIST_DIR) && zip -q $(BINARY)-$$platform.zip $(BINARY).exe && rm $(BINARY).exe; \
		else \
			cp $(TARGET_DIR)/$$platform/release/$(BINARY) $(DIST_DIR)/$(BINARY); \
			cd $(DIST_DIR) && tar -czf $(BINARY)-$$platform.tar.gz $(BINARY) && rm $(BINARY); \
		fi; \
		cd ..; \
	done
	@echo ""
	@echo "Archives created in $(DIST_DIR)/"
	@ls -la $(DIST_DIR)/

# Publish to GitHub releases
publish: dist
	@echo "Publishing v$(VERSION) to GitHub..."
	@if ! command -v gh &> /dev/null; then \
		echo "Error: gh CLI not found. Install from https://cli.github.com"; \
		exit 1; \
	fi
	@if ! gh auth status &> /dev/null; then \
		echo "Error: Not authenticated. Run: gh auth login"; \
		exit 1; \
	fi
	gh release create v$(VERSION) \
		--repo $(REPO) \
		--title "v$(VERSION)" \
		--generate-notes \
		$(DIST_DIR)/*.tar.gz $(DIST_DIR)/*.zip
	@echo ""
	@echo "Published v$(VERSION) to https://github.com/$(REPO)/releases/tag/v$(VERSION)"

# Publish to existing release (if release already exists)
publish-assets:
	@echo "Uploading assets to v$(VERSION)..."
	gh release upload v$(VERSION) \
		--repo $(REPO) \
		--clobber \
		$(DIST_DIR)/*.tar.gz $(DIST_DIR)/*.zip
	@echo "Assets uploaded"

clean:
	cargo clean
	rm -rf $(DIST_DIR)
	rm -f bin/$(BINARY)
