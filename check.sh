#!/usr/bin/bash
set -euox pipefail

cd "$(dirname "$0")"

if ! git diff HEAD --exit-code >/dev/null; then
    echo "error: uncommitted changes" >&2
    exit 1
fi

./regenerate-capnp-schema-code.sh
./regenerate-rpc-schema-code.sh

if ! git diff HEAD --exit-code >/dev/null; then
    echo "error: generated code changed" >&2
    exit 1
fi

cargo test --workspace
