default := "test-all"
set shell := ["bash", "-euo", "pipefail", "-c"]

# full test
test-all:
    cargo xtask test

# single target (e.g. `just test a`)
test target:
    cargo xtask test {{target}}

# run (e.g. `just run a` or `just run a 2`)
run bin case='':
    if [ -z '{{case}}' ]; then cargo xtask run {{bin}}; else cargo xtask run {{bin}} '{{case}}'; fi

# fetch (e.g. `just fetch abc322 a`)
fetch *args:
    cargo xtask fetch {{args}}

# fetch with overwrite
fetch-overwrite *args:
    cargo xtask fetch --overwrite {{args}}
