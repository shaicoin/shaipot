#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("graph_generator.h");

        type GraphGenerator;

        fn create_graph_generator(seed: u64, range: u64) -> UniquePtr<GraphGenerator>;
        fn next_random(self: Pin<&mut GraphGenerator>) -> u64;
    }
}

// pub use ffi::{create_graph_generator, GraphGenerator};
pub use ffi::*;
