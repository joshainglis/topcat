//! Import sources for extracting database objects from various formats.
//!
//! This module defines the [`ImportSource`] trait and related types for importing
//! database objects from different sources (pg_dump files, database introspection, etc.).
//!
//! # Architecture
//!
//! The import system uses a layered architecture:
//! 1. **Sources** (this module): Extract raw object data from various inputs
//! 2. **Handlers**: Process objects with type-specific logic
//! 3. **Output**: Write processed objects to files
//!
//! Sources produce [`RawObject`] instances that are then processed by handlers.

mod raw_object;
mod traits;

pub use raw_object::RawObject;
pub use traits::ImportSource;
