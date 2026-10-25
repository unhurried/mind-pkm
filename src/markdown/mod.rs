//! Markdown for the page editor, independent of the UI: splitting a page into
//! blocks, the inline content of a block, the editing rules, tables, and the
//! export to HTML. The page text is the single source of truth: everything
//! here reads it or returns edited text, never regenerates it from a tree.

pub mod content;
pub mod count;
pub mod edit;
pub mod export;
pub mod find;
pub mod format;
pub mod history;
pub mod html;
pub mod images;
pub mod page;
pub mod parse;
pub mod render;
pub mod table;

pub use parse::{Block, BlockKind, Document, ListItem};
