.PHONY: all build install install-bin install-data clean uninstall

BINDIR ?= /usr/local/bin
DATADIR ?= /usr/local/share
LOCALSTATEDIR ?= /var
RUNSTATEDIR ?= /run

POLARIS_BIN_PATH := $(BINDIR)/polaris
POLARIS_STATIC_DIR := $(DATADIR)/polaris
export POLARIS_WEB_DIR := $(POLARIS_STATIC_DIR)/web
export POLARIS_SWAGGER_DIR := $(POLARIS_STATIC_DIR)/swagger
export POLARIS_DB_DIR := $(LOCALSTATEDIR)/lib/polaris
export POLARIS_LOG_DIR := $(LOCALSTATEDIR)/log/polaris
export POLARIS_CACHE_DIR := $(LOCALSTATEDIR)/cache/polaris
export POLARIS_PID_DIR := $(RUNSTATEDIR)/polaris

all: build

build:
	cargo build --release

install: install-bin install-data

install-bin: build
	install -Dm755 ./target/release/polaris $(POLARIS_BIN_PATH)

install-data:
	install -d $(POLARIS_STATIC_DIR)
	cp -r ./web $(POLARIS_WEB_DIR)
	cp -r ./swagger $(POLARIS_SWAGGER_DIR)

clean:
	cargo clean

uninstall:
	rm $(POLARIS_BIN_PATH)
	rm -r $(POLARIS_STATIC_DIR)
	rm -r $(POLARIS_LOG_DIR)
	rm -r $(POLARIS_DB_DIR)
	rm -r $(POLARIS_CACHE_DIR)
	rm -r $(POLARIS_PID_DIR)
