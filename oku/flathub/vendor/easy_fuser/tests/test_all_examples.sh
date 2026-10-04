#!/bin/sh

for dir in examples/*/; do
    if [ -f "${dir}Cargo.toml" ]; then
    (
        echo "Running tests in ${dir}"
        if ! (cd "${dir}" && cargo test); then
        echo "Tests failed in ${dir}"
        exit 1
        fi
    )
    fi
done