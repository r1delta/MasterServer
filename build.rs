fn main() {
    println!("cargo:rerun-if-changed=src/schema/server.capnp");

    capnpc::CompilerCommand::new()
        .output_path("src/schema") 
        .src_prefix("src/schema")
        .file("src/schema/server.capnp")
        .import_path("src/schema")
        .default_parent_module(vec!["schema".into()]) // Add this line
        .run()
        .expect("schema compiler command");
}
