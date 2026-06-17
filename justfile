plugin := "signups-stats"
src_dir := "plugin-src/" + plugin
# cargo replaces '-' with '_' in the artifact name
artifact := "signups_stats"
module := src_dir + "/target/wasm32-unknown-unknown/release/" + artifact + ".wasm"
component := src_dir + "/build/" + plugin + ".wasm"
wasm_out := "plugins/" + plugin + "/" + plugin + ".wasm"

# Default: list recipes
default:
    @just --list

# Compile + componentize the plugin, but only if a source file is newer than the
# installed wasm (just has no native dep graph, so we gate on `find -newer`).
build-plugins:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f "{{wasm_out}}" ] && [ -z "$(find {{src_dir}}/src {{src_dir}}/Cargo.toml -newer {{wasm_out}} 2>/dev/null)" ]; then
        echo "{{plugin}}: up to date"; exit 0
    fi
    echo "{{plugin}}: building"
    cargo build --manifest-path {{src_dir}}/Cargo.toml --target wasm32-unknown-unknown --release
    mkdir -p "$(dirname {{component}})"
    wasm-tools component new {{module}} -o {{component}}

# Move built components into the served plugins/ tree (the distinct install step).
install-plugins: build-plugins
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$(dirname {{wasm_out}})"
    cp {{component}} {{wasm_out}}
    echo "installed {{wasm_out}}"

# Build + install plugins, then run Ledge (from the sibling checkout) against this project.
dev port="3000" authoring="3001": install-plugins
    cargo run --manifest-path ../ledge-cms/Cargo.toml -p ledge -- . --port {{port}} --authoring-port {{authoring}}

clean:
    rm -rf {{src_dir}}/build {{src_dir}}/target
