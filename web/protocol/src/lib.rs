#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate hex_database;

#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate cfg_if;

pub mod error;
pub mod objects;

#[cfg(target_arch = "wasm32")]
cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }   
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use objects::{Request, Answer, RequestAction, AnswerAction, PacketId};
pub use error::Error;
