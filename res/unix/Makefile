POLARIS_BIN_DIR := /usr/bin/polaris
POLARIS_STATIC_DIR := /usr/share/polaris
POLARIS_LOG_DIR := /var/log/polaris
POLARIS_DB_DIR := /var/lib/polaris
POLARIS_CACHE_DIR := /var/cache/polaris
POLARIS_PID_DIR := /run/polaris

all: build

build:
	cargo build --release

install: build
	install -d $(POLARIS_BIN_DIR)
	install -d $(POLARIS_STATIC_DIR)
	install ./target/release/polaris $(POLARIS_BIN_DIR)
	cp -r ./web $(POLARIS_STATIC_DIR)
	cp -r ./swagger $(POLARIS_STATIC_DIR)
	@echo "Polaris installation complete!"

clean:
	cargo clean

uninstall:
	rm -r $(POLARIS_BIN_DIR)
	rm -r $(POLARIS_STATIC_DIR)
	rm -r $(POLARIS_LOG_DIR)
	rm -r $(POLARIS_DB_DIR)
	rm -r $(POLARIS_CACHE_DIR)
	rm -r $(POLARIS_PID_DIR)
