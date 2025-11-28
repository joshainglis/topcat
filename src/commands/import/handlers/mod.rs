//! Type-specific handlers for processing imported database objects.
//!
//! This module provides the handler system for processing [`RawObject`] instances
//! with type-specific logic. Each PostgreSQL object type has a corresponding handler
//! that implements the composable traits defined in [`traits`].
//!
//! # Architecture
//!
//! Handlers are organized by category:
//! - `schema_objects/`: Core schema objects (Table, View, Schema, etc.)
//! - `types/`: Type system objects (Type, Domain, Collation)
//! - `routines/`: Callable objects (Function, Procedure, Aggregate)
//! - `attachments/`: Objects that attach to parents (Index, Trigger, etc.)
//! - `fts/`: Full-text search objects
//! - `fdw/`: Foreign data wrapper objects
//! - `operators/`: Operators and casts
//! - `replication/`: Replication objects
//! - `security/`: Security objects (ACL, SecurityLabel)
//! - `global/`: Database-level objects (Language, EventTrigger)
//!
//! # Usage
//!
//! ```ignore
//! use crate::commands::import::handlers::HandlerRegistry;
//!
//! let registry = HandlerRegistry::new();
//! let handler = registry.get(&ObjectType::Table).unwrap();
//! let deps = handler.extract_pattern_dependencies(&raw_object.content);
//! ```

mod registry;
mod traits;

pub use traits::{
    Categorizer, Configurable, DependencyExtractor, ObjectHandler, PatternProvider, Renderer,
};

// Category modules - migrated handlers organized by type
pub mod attachments;
pub mod fdw;
pub mod fts;
pub mod operators;
pub mod replication;
pub mod routines;
pub mod schema_objects;
pub mod types;

// Remaining category modules will be added as they are migrated:
// pub mod security;
// pub mod global;
