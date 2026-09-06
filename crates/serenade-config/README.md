# serenade-config

Layered configuration: dotenv, package TOML/YAML, env overlays, and `${VAR}`
interpolation into the DI parameter bag.

Prefer `config/packages/*.toml` for new apps. Secrets come from the environment
or operator files, never hard-coded.
