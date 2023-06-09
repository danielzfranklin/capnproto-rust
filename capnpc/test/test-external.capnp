@0xf65f5a25101789bb;

using Rust = import "rust.capnp";
using External = import "./external-crate/external.capnp";

$Rust.imports([
  (path = "./external-crate/external.capnp", crate = "external_crate")
]);

# The test case is that this builds. This ensure we're able to refer to a struct
# (external_capnp::opts) in the generated code.
struct UseExternalAnnotation $External.annot(field = "foo") {
  field @0 :Text;
}
