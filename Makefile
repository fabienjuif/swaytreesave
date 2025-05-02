default:
	@cargo fmt --all
	@cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings
