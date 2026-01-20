//! mcpsol-core: Framework-agnostic MCP schema generation for Solana
//!
//! This crate provides the core types and utilities for generating
//! MCP (Model Context Protocol) schemas that work with any Solana framework.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec, format};

mod discriminator;
mod schema;
mod json;

pub use discriminator::*;
pub use schema::*;
pub use json::{
    // Compact schema (backwards compatible)
    generate_compact_schema,
    generate_schema_bytes,
    estimate_schema_size,
    estimate_single_tool_size,
    // Paginated verbose schema (full descriptions)
    generate_paginated_schema,
    generate_paginated_schema_bytes,
};

/// MCP protocol version
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Universal list_tools discriminator
/// sha256("global:list_tools")[0..8]
pub const LIST_TOOLS_DISCRIMINATOR: [u8; 8] = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0];

/// Maximum size for return_data on Solana (1024 bytes)
pub const MAX_RETURN_DATA_SIZE: usize = 1024;
