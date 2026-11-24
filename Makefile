.PHONY: metrics test lint readme check wasm
metrics:
	cd rust && cargo run --release --bin metrics
test:
	cd rust && cargo test --release
lint:
	cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings
readme:
	python3 scripts/readme_numbers.py --write
check:
	python3 scripts/readme_numbers.py --check
wasm:
	sh web/build.sh
