fn main() {
    capnpc::CompilerCommand::new()
        .file("../../test-util/test.capnp")
        .import_path("../../capnpc")
        .src_prefix("../test/")
        .run()
        .expect("compiling schema");
}
