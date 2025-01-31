.PHONY: all build build-system build-xdg cargo-build clean preview preview-system preview-xdg list-paths install install-bin install-data install-system install-xdg uninstall uninstall-bin uninstall-data uninstall-system uninstall-xdg

UID := $(shell id -u)

PREFIX ?= /usr/local
EXEC_PREFIX ?= $(PREFIX)
BINDIR ?= $(EXEC_PREFIX)/bin
DATAROOTDIR ?= $(PREFIX)/share
DATADIR ?= $(DATAROOTDIR)
SYSCONFDIR ?= $(PREFIX)/etc
LOCALSTATEDIR ?= $(PREFIX)/var
RUNSTATEDIR ?= $(LOCALSTATEDIR)/run
%-system: POLARIS_BIN_PATH := $(BINDIR)/polaris
%-system: export POLARIS_WEB_DIR := $(DATADIR)/polaris/web
%-system: export POLARIS_CONFIG_DIR := $(SYSCONFDIR)/polaris
%-system: export POLARIS_DATA_DIR := $(LOCALSTATEDIR)/lib/polaris
%-system: export POLARIS_DB_DIR := $(LOCALSTATEDIR)/lib/polaris
%-system: export POLARIS_LOG_DIR := $(LOCALSTATEDIR)/log/polaris
%-system: export POLARIS_CACHE_DIR := $(LOCALSTATEDIR)/cache/polaris
%-system: export POLARIS_PID_DIR := $(RUNSTATEDIR)/polaris

XDG_CACHE_HOME ?= $(HOME)/.cache
XDG_CONFIG_HOME ?= $(HOME)/.config
XDG_DATA_HOME ?= $(HOME)/.local/share
XDG_BINDIR ?= $(HOME)/.local/bin
XDG_DATADIR ?= $(XDG_DATA_HOME)/polaris
XDG_CACHEDIR ?= $(XDG_CACHE_HOME)/polaris
XDG_CONFIGDIR ?= $(XDG_CONFIG_HOME)/polaris
ifdef $(XDG_RUNTIME_DIR)
XDG_PIDDIR ?= $(XDG_RUNTIME_DIR)/polaris
else
XDG_PIDDIR ?= /tmp/polaris-$(UID)
endif
%-xdg: POLARIS_BIN_PATH := $(XDG_BINDIR)/polaris
%-xdg: export POLARIS_WEB_DIR := $(XDG_DATADIR)/web
%-xdg: export POLARIS_CONFIG_DIR := $(XDG_CONFIGDIR)
%-xdg: export POLARIS_DATA_DIR := $(XDG_DATADIR)
%-xdg: export POLARIS_DB_DIR := $(XDG_DATADIR)
%-xdg: export POLARIS_LOG_DIR := $(XDG_CACHEDIR)
%-xdg: export POLARIS_CACHE_DIR := $(XDG_CACHEDIR)
%-xdg: export POLARIS_PID_DIR := $(XDG_PIDDIR)

# Build

build-system: cargo-build
build-xdg: cargo-build
build: build-system
all: build-system

cargo-build:
	cargo build --release

clean:
	cargo clean

# Preview

preview-system: list-paths
preview-xdg: list-paths
preview: preview-system

list-paths:
	$(info POLARIS_BIN_PATH is $(POLARIS_BIN_PATH))
	$(info POLARIS_WEB_DIR is $(POLARIS_WEB_DIR))
	$(info POLARIS_CONFIG_DIR is $(POLARIS_CONFIG_DIR))
	$(info POLARIS_DATA_DIR is $(POLARIS_DATA_DIR))
	$(info POLARIS_DB_DIR is $(POLARIS_DB_DIR))
	$(info POLARIS_LOG_DIR is $(POLARIS_LOG_DIR))
	$(info POLARIS_CACHE_DIR is $(POLARIS_CACHE_DIR))
	$(info POLARIS_PID_DIR is $(POLARIS_PID_DIR))

# Install

install-system: install-bin install-data
install-xdg: install-bin install-data
install: install-system

install-bin: cargo-build
	install -Dm755 ./target/release/polaris $(POLARIS_BIN_PATH)

install-data:
	install -d $(POLARIS_WEB_DIR)
	cp -rT ./web $(POLARIS_WEB_DIR)

# Uninstall

uninstall-system: uninstall-bin uninstall-data
uninstall-xdg: uninstall-bin uninstall-data
uninstall: uninstall-system

uninstall-bin:
	rm $(POLARIS_BIN_PATH)

uninstall-data:
	rm -rf $(POLARIS_WEB_DIR)
	rm -rf $(POLARIS_CONFIG_DIR)
	rm -rf $(POLARIS_DATA_DIR)
	rm -rf $(POLARIS_DB_DIR)
	rm -rf $(POLARIS_LOG_DIR)
	rm -rf $(POLARIS_CACHE_DIR)
	rm -rf $(POLARIS_PID_DIR)
