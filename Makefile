PREFIX ?= /usr/local
DESTDIR ?=
NEUSWC_REPO ?= https://git.sr.ht/~shrub900/neuswc
NEUSWC_DIR ?= deps/neuswc
NEUSWC_BUILDDIR ?= $(NEUSWC_DIR)/builddir

CARGO ?= cargo
MESON ?= meson

.PHONY: all clean install uninstall libswc

all: libswc
	SWC_LIBDIR=$(CURDIR)/$(NEUSWC_BUILDDIR)/libswc $(CARGO) build --release

libswc: $(NEUSWC_BUILDDIR)/build.ninja
	ninja -C $(NEUSWC_BUILDDIR)

$(NEUSWC_DIR):
	git clone $(NEUSWC_REPO) $(NEUSWC_DIR)

$(NEUSWC_BUILDDIR)/build.ninja: $(NEUSWC_DIR)
	$(MESON) setup $(NEUSWC_DIR) $(NEUSWC_BUILDDIR)

install: all
	install -Dm755 target/release/wm15 $(DESTDIR)$(PREFIX)/bin/wm15
	install -Dm644 $(NEUSWC_BUILDDIR)/libswc/libswc.so $(DESTDIR)$(PREFIX)/lib/libswc.so
	ldconfig 2>/dev/null || true

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/wm15
	rm -f $(DESTDIR)$(PREFIX)/lib/libswc.so
	ldconfig 2>/dev/null || true

clean:
	rm -rf target
	rm -rf $(NEUSWC_BUILDDIR)
