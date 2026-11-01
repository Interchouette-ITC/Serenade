# User-facing aliases (scaffolding CLI + demo console). Prefer these over raw cargo run.

.PHONY: serenade serenade-tui tui recipe-list console demo serenade-html html html-all myfeed serenade-spa spa spa-all demo-spa serenade-universal universal serenade-ssg ssg ssg-all demo-ssg demo-universal

# Scaffolding CLI: `make serenade` or `make serenade ARGS='new demo --path /tmp'`
serenade:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade -- $(ARGS)

# Guided recipe picker (ratatui). Extra flags via ARGS, e.g. ARGS='--no-cargo'
serenade-tui tui:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade -- tui $(ARGS)

recipe-list:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade -- recipe list

# Demo app console: `make console ARGS='serenade:about'` or `make console ARGS='--interactive'`
console:
	cd $(ROOT) && $(CARGO) run -p serenade-demo-app --bin console -- $(ARGS)

# Demo HTTP binary
demo:
	cd $(ROOT) && $(CARGO) run -p serenade-demo-app --bin serenade-demo-app -- $(ARGS)

# HTML mode: run the default, named, or all registered HTML apps.
serenade-html:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade-html -- $(ARGS)

html:
	$(MAKE) serenade-html

html-all:
	$(MAKE) serenade-html ARGS=--all

# Legacy alias: `make myfeed` → same as `make html`
myfeed: html

# SPA demo: build every registered mode=spa app with `ARGS=--all`.
serenade-spa:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade-spa -- $(ARGS)

spa:
	$(MAKE) serenade-spa

spa-all:
	$(MAKE) serenade-spa ARGS=--all

demo-spa: spa

serenade-universal:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade-universal -- $(ARGS)

universal:
	$(MAKE) serenade-universal

demo-universal: universal

serenade-ssg:
	cd $(ROOT) && $(CARGO) run -p serenade-cli --bin serenade-ssg -- $(ARGS)

ssg:
	$(MAKE) serenade-ssg

ssg-all:
	$(MAKE) serenade-ssg ARGS=--all

demo-ssg: ssg
