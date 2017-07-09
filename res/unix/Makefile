POLARIS_BIN_DIR := ~/.local/bin/polaris
POLARIS_DATA_DIR := ~/.local/share/polaris

all: build

build:
	cargo build --release

install: build
	install -d $(POLARIS_BIN_DIR)
	install -d $(POLARIS_DATA_DIR)
	install ./target/release/polaris $(POLARIS_BIN_DIR)
	cp -r ./web $(POLARIS_DATA_DIR)
	@echo "Polaris installation complete!"

clean:
	cargo clean

uninstall:
	rm -r $(POLARIS_BIN_DIR)
	rm -r $(POLARIS_DATA_DIR)
