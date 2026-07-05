.PHONY: fix release
fix:
	@cargo fmt --all
	@cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings

# make release VERSION=x.y.z
# Bumps the package version (single source of truth for the reported --version),
# refreshes Cargo.lock, commits, tags vX.Y.Z so the tag and manifest can't drift,
# pushes both, and opens a GitHub release with auto-generated notes.
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
	@git push
	@git push origin "v$(VERSION)"
	@gh release create "v$(VERSION)" --title "v$(VERSION)" --generate-notes
	@echo "released v$(VERSION)"
