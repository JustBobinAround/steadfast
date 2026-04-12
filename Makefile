build:
	cargo build --workspace
test:
	cargo test --workspace

docs:
	cargo doc --workspace --document-private-items

open-docs:
	cargo doc --workspace --document-private-items --open

	
