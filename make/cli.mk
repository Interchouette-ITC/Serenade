# User-facing aliases (scaffolding CLI + demo console). Prefer these over raw cargo run.

.PHONY: serenade serenade-tui tui recipe-list console demo

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
	cd $(ROOT) && $(CARGO) run -p serenade-demo-app -- $(ARGS)
