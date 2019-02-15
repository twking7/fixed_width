#!/bin/bash

set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

channel() {
   if [ "${TRAVIS_RUST_VERSION}" = "${CHANNEL}" ]; then
       pwd
       (set -x; cargo "$@")
   fi
}

if [ -n "${CLIPPY}" ]; then
    # cached installation will not work on a later nightly
    if [ -n "${TRAVIS}" ] && ! cargo install clippy --debug --force; then
        echo "COULD NOT COMPILE CLIPPY, IGNORING CLIPPY TESTS"
        exit
    fi

    cd "$DIR/fixed_width"
    cargo clippy --features 'rc unstable' -- -Dclippy

    cd "$DIR/fixed_width_derive"
    cargo clippy -- -Dclippy
else
    for CHANNEL in beta stable nightly; do
      cd "$DIR"
      cargo clean
      channel build
      channel test
    done
fi
