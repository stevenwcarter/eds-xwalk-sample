plugin := "signups-stats"
src_dir := "plugin-src/" + plugin
wasm_out := "plugins/" + plugin + "/" + plugin + ".wasm"

# Default: list recipes
default:
    @just --list

# Build + install the plugin via `ledge plugin build`, but only if a source file
# is newer than the installed wasm (just has no native dep graph, so we gate on
# `find -newer`; `ledge plugin build` has no up-to-date check of its own).
build-plugins:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f "{{wasm_out}}" ] && [ -z "$(find {{src_dir}}/src {{src_dir}}/Cargo.toml -newer {{wasm_out}} 2>/dev/null)" ]; then
        echo "{{plugin}}: up to date"; exit 0
    fi
    cargo run --manifest-path ../ledge-cms/Cargo.toml -p ledge -- \
        plugin build --name {{plugin}} --project . --manifest-path {{src_dir}}/Cargo.toml

# `ledge plugin build` already installs into plugins/<name>/, so the distinct
# install step is now just the up-to-date-gated build above, kept under its old
# name for the `dev` recipe below.
install-plugins: build-plugins

# Build + install plugins, then run Ledge (from the sibling checkout) against this project.
dev port="3000" authoring="3001": install-plugins
    cargo run --manifest-path ../ledge-cms/Cargo.toml -p ledge -- . --port {{port}} --authoring-port {{authoring}}

clean:
    rm -rf {{src_dir}}/target
