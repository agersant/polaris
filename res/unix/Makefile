.PHONY: all build install install-bin install-data clean uninstall

POLARIS_BIN_DIR := /usr/bin
POLARIS_STATIC_DIR := /usr/share/polaris
POLARIS_LOG_DIR := /var/log/polaris
POLARIS_DB_DIR := /var/lib/polaris
POLARIS_CACHE_DIR := /var/cache/polaris
POLARIS_PID_DIR := /run/polaris

all: build

build:
	cargo build --release

install: install-bin install-data

install-bin: build
	install -Dm755 ./target/release/polaris $(POLARIS_BIN_DIR)/polaris

install-data:
	install -d $(POLARIS_STATIC_DIR)
	cp -r ./web $(POLARIS_STATIC_DIR)/web
	cp -r ./swagger $(POLARIS_STATIC_DIR)/swagger

clean:
	cargo clean

uninstall:
	rm $(POLARIS_BIN_DIR)/polaris
	rm -r $(POLARIS_STATIC_DIR)
	rm -r $(POLARIS_LOG_DIR)
	rm -r $(POLARIS_DB_DIR)
	rm -r $(POLARIS_CACHE_DIR)
	rm -r $(POLARIS_PID_DIR)
