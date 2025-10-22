#[cfg(feature = "use_jemalloc")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub mod ffi_bindings;
pub mod seed;
pub mod wallet;
pub use ffi_bindings::free_string;
pub use seed::{generate_seed_phrase, generate_seed_phrase_24};
pub use wallet::{
    derive_and_save_wallet
};



