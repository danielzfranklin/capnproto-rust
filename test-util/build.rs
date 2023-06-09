fn main() {
    capnpc::CompilerCommand::new()
        .import_path("../capnpc")
        .file("test.capnp")
        .run()
        .expect("compiling schema");
}
