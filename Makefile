CARGO := cargo
BIN_NAME := tetris

INSTALL_DIR := /usr/local/bin
CONFIG_DIR := $(HOME)/.config

all: build

build:
	$(CARGO) build --release

install: build
	sudo cp target/release/$(BIN_NAME) $(INSTALL_DIR)
	mkdir -p $(CONFIG_DIR)
	cp conf.ini $(CONFIG_DIR)/tetris.ini
	@echo "Successfully installed to $(INSTALL_DIR)/$(BIN_NAME)"

uninstall:
	sudo rm -f $(INSTALL_DIR)/$(BIN_NAME)
	rm -f $(CONFIG_DIR)/tetris.ini
	@echo "Successfully uninstalled from $(INSTALL_DIR)/$(BIN_NAME)"

clean:
	$(CARGO) clean

