#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust:capnp-json/src/ -Icapnpc/ capnp-json/json.capnp --src-prefix capnp-json/
