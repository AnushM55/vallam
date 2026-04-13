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

install:
	@test -x target/release/vallam || $(MAKE) all
	@test -x $(NEUSWC_BUILDDIR)/extra/swcsnap || $(MAKE) libswc
	@test -f $(NEUSWC_BUILDDIR)/libswc/libswc.so || $(MAKE) libswc
	install -Dm755 target/release/vallam $(DESTDIR)$(PREFIX)/bin/vallam
	install -Dm755 scripts/vallam-wayrec $(DESTDIR)$(PREFIX)/bin/vallam-wayrec
	install -Dm755 $(NEUSWC_BUILDDIR)/extra/swcsnap $(DESTDIR)$(PREFIX)/bin/swcsnap
	install -Dm644 $(NEUSWC_BUILDDIR)/libswc/libswc.so $(DESTDIR)$(PREFIX)/lib/libswc.so
	ldconfig 2>/dev/null || true

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/vallam
	rm -f $(DESTDIR)$(PREFIX)/bin/vallam-wayrec
	rm -f $(DESTDIR)$(PREFIX)/bin/swcsnap
	rm -f $(DESTDIR)$(PREFIX)/lib/libswc.so
	ldconfig 2>/dev/null || true

clean:
	rm -rf target
	rm -rf $(NEUSWC_BUILDDIR)
