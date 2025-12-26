fn main() {
    cxx_build::bridge("src/graph_bridge.rs")
        .file("src/cpp/graph_generator.cpp")
        .include("src/cpp")
        .std("c++17")
        .compile("graph_generator");

    println!("cargo:rerun-if-changed=src/graph_bridge.rs");
    println!("cargo:rerun-if-changed=src/cpp/graph_generator.cpp");
    println!("cargo:rerun-if-changed=src/cpp/graph_generator.h");
}
