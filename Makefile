INSTALL_DIR ?= $(HOME)/bin

test:
	cargo test

compile:
	cargo build --release
	mkdir -p $(INSTALL_DIR)
	cp target/release/dopper $(INSTALL_DIR)/