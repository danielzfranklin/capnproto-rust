#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust-bootstrap:capnp/src --src-prefix capnpc/ capnpc/rust.capnp
