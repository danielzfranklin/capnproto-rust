#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust:capnp-rpc/src capnp-rpc/schema/rpc.capnp capnp-rpc/schema/rpc-twoparty.capnp --src-prefix capnp-rpc/schema/
