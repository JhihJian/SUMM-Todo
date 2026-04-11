pub mod cli;
pub mod commands;
pub mod db;
pub mod output;
pub mod time_parse;

// Re-export core types so existing `use crate::task::`, `use crate::error::`,
// and `use crate::id::` imports continue to work unchanged.
pub mod error {
    pub use todo_core::error::*;
}

pub mod id {
    pub use todo_core::id::*;
}

pub mod task {
    pub use todo_core::task::*;
}
