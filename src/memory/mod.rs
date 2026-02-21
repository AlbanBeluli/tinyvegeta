//! Memory system - three-layer memory with persistence.

pub mod lock;
pub mod sqlite;
pub mod store;

pub use store::{
    ensure_memory_dirs, Memory,
    MemoryEntry, MemoryScope,
};
