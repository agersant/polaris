.PHONY: all build clean preview preview-system preview-xdg list-paths install install-bin install-data install-system install-xdg uninstall uninstall-bin uninstall-data uninstall-system uninstall-xdg

%-system: PREFIX ?= /usr/local
%-system: EXEC_PREFIX ?= $(PREFIX)
%-system: BINDIR ?= $(EXEC_PREFIX)/bin
%-system: DATAROOTDIR ?= $(PREFIX)/share
%-system: DATADIR ?= $(DATAROOTDIR)
%-system: LOCALSTATEDIR ?= $(PREFIX)/var
%-system: RUNSTATEDIR ?= $(LOCALSTATEDIR)/run
%-system: POLARIS_BIN_PATH = $(BINDIR)/polaris
%-system: export POLARIS_WEB_DIR =  $(DATADIR)/polaris/web
%-system: export POLARIS_SWAGGER_DIR =  $(DATADIR)/polaris/swagger
%-system: export POLARIS_DB_DIR = $(LOCALSTATEDIR)/lib/polaris
%-system: export POLARIS_LOG_DIR = $(LOCALSTATEDIR)/log/polaris
%-system: export POLARIS_CACHE_DIR = $(LOCALSTATEDIR)/cache/polaris
%-system: export POLARIS_PID_DIR = $(RUNSTATEDIR)/polaris

ifdef $(XDG_RUNTIME_DIR)
XDG_PID_DIR ?= $(XDG_RUNTIME_DIR)/polaris
else
XDG_PID_DIR ?= /tmp/polaris-$(UID)
endif
%-xdg: XDG_CACHE_HOME ?= $(HOME)/.cache
%-xdg: XDG_DATA_HOME ?= $(HOME)/.local/share
%-xdg: BINDIR ?= $(HOME)/.local/bin
%-xdg: DATADIR ?= $(XDG_DATA_HOME)/polaris
%-xdg: CACHEDIR ?= $(XDG_PID_DIR)
%-xdg: POLARIS_BIN_PATH = $(BINDIR)/polaris
%-xdg: export POLARIS_WEB_DIR =  $(DATADIR)/web
%-xdg: export POLARIS_SWAGGER_DIR =  $(DATADIR)/swagger
%-xdg: export POLARIS_DB_DIR = $(DATADIR)
%-xdg: export POLARIS_LOG_DIR = $(CACHEDIR)
%-xdg: export POLARIS_CACHE_DIR = $(CACHEDIR)
%-xdg: export POLARIS_PID_DIR = $(CACHEDIR)

# Build

all: build

build:
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
	$(info POLARIS_SWAGGER_DIR is $(POLARIS_SWAGGER_DIR))
	$(info POLARIS_DB_DIR is $(POLARIS_DB_DIR))
	$(info POLARIS_LOG_DIR is $(POLARIS_LOG_DIR))
	$(info POLARIS_CACHE_DIR is $(POLARIS_CACHE_DIR))
	$(info POLARIS_PID_DIR is $(POLARIS_PID_DIR))

# Install

install-system: install-bin install-data
install-xdg: install-bin install-data
install: install-system

install-bin: build
	install -Dm755 ./target/release/polaris $(POLARIS_BIN_PATH)

install-data:
	install -d $(POLARIS_WEB_DIR)
	install -d $(POLARIS_SWAGGER_DIR)
	cp -rT ./web $(POLARIS_WEB_DIR)
	cp -rT ./swagger $(POLARIS_SWAGGER_DIR)

# Uninstall

uninstall-system: uninstall-bin uninstall-data
uninstall-xdg: uninstall-bin uninstall-data
uninstall: uninstall-system

uninstall-bin:
	rm $(POLARIS_BIN_PATH)

uninstall-data:
	rm -r $(POLARIS_WEB_DIR)
	rm -r $(POLARIS_SWAGGER_DIR)
	rm -r $(POLARIS_DB_DIR)
	rm -r $(POLARIS_LOG_DIR)
	rm -r $(POLARIS_CACHE_DIR)
	rm -r $(POLARIS_PID_DIR)
