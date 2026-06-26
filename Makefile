.PHONY: fix release
fix:
	@cargo fmt --all
	@cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings

# make release VERSION=x.y.z
# Bumps the package version (single source of truth for the reported --version),
# refreshes Cargo.lock, commits, and tags vX.Y.Z so the tag and manifest can't drift.
release:
ifndef VERSION
	$(error VERSION is required, e.g. `make release VERSION=0.5.0`)
endif
	@test -z "$$(git status --porcelain)" || { echo "working tree is dirty; commit or stash first"; exit 1; }
	@sed -i 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@cargo check --quiet                # rewrites Cargo.lock with the new version and verifies it builds
	@git add Cargo.toml Cargo.lock
	@git commit -m ":bookmark: release v$(VERSION)"
	@git tag "v$(VERSION)"
	@echo "tagged v$(VERSION) — push with: git push && git push origin v$(VERSION)"
