mod canon;
#[cfg(not(target_arch = "wasm32"))]
mod library;
#[cfg(not(target_arch = "wasm32"))]
mod playability;
mod reference;
mod scope;

pub use canon::{Canon, Difficulty, GameMode};
#[cfg(not(target_arch = "wasm32"))]
pub use library::{ScriptureLibrary, Verse};
#[cfg(not(target_arch = "wasm32"))]
pub use reference::Reference;
pub use reference::{BookInfo, ChapterRef};
pub use scope::{BookScope, CanonScope, GameScope};
