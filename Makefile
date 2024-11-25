build:
	cargo build --release

run_example:
	export RAVEDUDE_PORT=/dev/ttyUSB0
	cargo run --example dumpinfo
