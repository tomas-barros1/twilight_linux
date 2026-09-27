PREFIX  ?= $(prefix)
DESTDIR ?=

BINDIR      := $(PREFIX)/bin
DATADIR     := $(PREFIX)/share
APPLICATIONSDIR := $(DATADIR)/applications
METAINFODIR := $(DATADIR)/metainfo
ICONDIR     := $(DATADIR)/icons/hicolor/scalable/apps
SYSTEMDUSERUNITDIR := $(PREFIX)/lib/systemd/user

APPID       := org.twilight_pc.Twilight
BINARY      := twilight-pc
CARGO_TARGET := target/release/$(BINARY)

.PHONY: all build release clean install uninstall check test

all: release

release:
	cargo build --release

build: release

check:
	cargo check

test:
	cargo test

clean:
	cargo clean

install: release
	install -d "$(DESTDIR)$(BINDIR)"
	install -d "$(DESTDIR)$(APPLICATIONSDIR)"
	install -d "$(DESTDIR)$(METAINFODIR)"
	install -d "$(DESTDIR)$(ICONDIR)"
	install -d "$(DESTDIR)$(SYSTEMDUSERUNITDIR)"
	install -Dm755 "$(CARGO_TARGET)" "$(DESTDIR)$(BINDIR)/$(BINARY)"
	install -Dm644 "data/$(APPID).desktop" \
		"$(DESTDIR)$(APPLICATIONSDIR)/$(APPID).desktop"
	install -Dm644 "data/$(APPID).metainfo.xml" \
		"$(DESTDIR)$(METAINFODIR)/$(APPID).metainfo.xml"
	install -Dm644 "data/icons/$(APPID).svg" \
		"$(DESTDIR)$(ICONDIR)/$(APPID).svg"
	install -Dm644 "data/twilight-sunsetr.service" \
		"$(DESTDIR)$(SYSTEMDUSERUNITDIR)/twilight-sunsetr.service"
	@echo
	@echo "Instalado em $(DESTDIR)$(PREFIX)"
	@echo "  bin:      $(BINDIR)/$(BINARY)"
	@echo "  desktop:  $(APPLICATIONSDIR)/$(APPID).desktop"
	@echo "  metainfo: $(METAINFODIR)/$(APPID).metainfo.xml"
	@echo "  icon:     $(ICONDIR)/$(APPID).svg"
	@echo "  systemd:  $(SYSTEMDUSERUNITDIR)/twilight-sunsetr.service"
	@echo
	@echo "Depois: gtk-update-icon-cache -f -t $(DATADIR)/icons/hicolor 2>/dev/null || true"
	@echo "        update-desktop-database $(APPLICATIONSDIR) 2>/dev/null || true"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/$(BINARY)"
	rm -f "$(DESTDIR)$(APPLICATIONSDIR)/$(APPID).desktop"
	rm -f "$(DESTDIR)$(METAINFODIR)/$(APPID).metainfo.xml"
	rm -f "$(DESTDIR)$(ICONDIR)/$(APPID).svg"
	rm -f "$(DESTDIR)$(SYSTEMDUSERUNITDIR)/twilight-sunsetr.service"
	@echo "Removido de $(DESTDIR)$(PREFIX)"
