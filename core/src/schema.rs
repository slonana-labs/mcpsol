//! MCP schema types for tool and resource definitions
//!
//! This module provides compact schema types optimized for on-chain storage
//! within Solana's 1024-byte `return_data` limit. For full MCP-compliant
//! schemas with JSON Schema support, see the SDK's `mcp` module.
//!
//! # Architecture
//!
//! - **Compact format**: Uses abbreviated keys (`n`, `d`, `p`, `r`) to minimize size
//! - **Paginated format**: One tool per page with full descriptions for AI agents
//! - **Discriminators**: SHA256-based instruction/account identification

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

/// A complete MCP program schema for on-chain tool discovery.
///
/// This is the compact schema format designed to fit within Solana's
/// `return_data` limit of 1024 bytes. Use [`McpSchemaBuilder`] to construct.
///
/// # Example
///
/// ```
/// use mcpsol_core::{McpSchemaBuilder, McpToolBuilder, ArgType};
///
/// let schema = McpSchemaBuilder::new("my_program")
///     .add_tool(
///         McpToolBuilder::new("transfer")
///             .description("Transfer tokens")
///             .signer_writable("from")
///             .writable("to")
///             .arg("amount", ArgType::U64)
///             .build()
///     )
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct McpSchema {
    /// Program name
    pub name: String,
    /// Available tools (instructions)
    pub tools: Vec<McpTool>,
}

/// An MCP tool (instruction) definition.
///
/// Represents a single Solana instruction with its metadata for AI discovery.
/// The discriminator is auto-generated from the tool name using SHA256.
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Tool/instruction name
    pub name: String,
    /// Human-readable description for AI agents
    pub description: Option<String>,
    /// 8-byte instruction discriminator (SHA256 of "global:{name}")
    pub discriminator: [u8; 8],
    /// Required accounts for this instruction
    pub accounts: Vec<McpAccountMeta>,
    /// Instruction arguments (serialized after discriminator)
    pub args: Vec<McpArg>,
}

/// Account metadata for a tool.
///
/// Describes a required account for an instruction, including its
/// signer/writable requirements and optional description for AI agents.
#[derive(Debug, Clone)]
pub struct McpAccountMeta {
    /// Account name (used in compact schema with suffix)
    pub name: String,
    /// Human-readable description for AI agents
    pub description: Option<String>,
    /// Whether this account must sign the transaction
    pub is_signer: bool,
    /// Whether this account's data is modified
    pub is_writable: bool,
}

impl McpAccountMeta {
    /// Get the suffix for compact schema format
    /// _s = signer, _w = writable, _sw = both
    pub const fn suffix(&self) -> &'static str {
        match (self.is_signer, self.is_writable) {
            (true, true) => "_sw",
            (true, false) => "_s",
            (false, true) => "_w",
            (false, false) => "",
        }
    }
}

/// Argument definition for a tool.
///
/// Describes an instruction argument with its type for proper serialization.
#[derive(Debug, Clone)]
pub struct McpArg {
    /// Argument name
    pub name: String,
    /// Human-readable description for AI agents
    pub description: Option<String>,
    /// Argument type for serialization
    pub arg_type: ArgType,
}

/// Supported argument types for instruction parameters.
///
/// Maps to Solana/Rust primitive types for proper serialization.
/// New variants may be added in future versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArgType {
    /// Unsigned 8-bit integer
    U8,
    /// Unsigned 16-bit integer
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer (most common for amounts)
    U64,
    /// Unsigned 128-bit integer
    U128,
    /// Signed 8-bit integer
    I8,
    /// Signed 16-bit integer
    I16,
    /// Signed 32-bit integer
    I32,
    /// Signed 64-bit integer
    I64,
    /// Signed 128-bit integer
    I128,
    /// Boolean value
    Bool,
    /// 32-byte public key
    Pubkey,
    /// Variable-length string (Borsh-encoded: 4-byte length prefix)
    String,
    /// Variable-length bytes (Borsh-encoded: 4-byte length prefix)
    Bytes,
}

impl ArgType {
    /// Get the compact type name for schema
    pub const fn compact_name(&self) -> &'static str {
        match self {
            ArgType::U8 => "u8",
            ArgType::U16 => "u16",
            ArgType::U32 => "u32",
            ArgType::U64 => "u64",
            ArgType::U128 => "u128",
            ArgType::I8 => "i8",
            ArgType::I16 => "i16",
            ArgType::I32 => "i32",
            ArgType::I64 => "i64",
            ArgType::I128 => "i128",
            ArgType::Bool => "bool",
            ArgType::Pubkey => "pubkey",
            ArgType::String => "str",
            ArgType::Bytes => "bytes",
        }
    }

    /// Parse from Rust type string
    pub fn from_rust_type(ty: &str) -> Self {
        match ty {
            "u8" => ArgType::U8,
            "u16" => ArgType::U16,
            "u32" => ArgType::U32,
            "u64" => ArgType::U64,
            "u128" => ArgType::U128,
            "i8" => ArgType::I8,
            "i16" => ArgType::I16,
            "i32" => ArgType::I32,
            "i64" => ArgType::I64,
            "i128" => ArgType::I128,
            "bool" => ArgType::Bool,
            t if t.contains("Pubkey") => ArgType::Pubkey,
            t if t.starts_with("Vec<u8>") || t.starts_with("[u8;") => ArgType::Bytes,
            _ => ArgType::String,
        }
    }
}

/// Builder for creating MCP schemas programmatically.
///
/// # Example
///
/// ```
/// use mcpsol_core::{McpSchemaBuilder, McpToolBuilder};
///
/// let schema = McpSchemaBuilder::new("my_program")
///     .add_tool(McpToolBuilder::new("init").build())
///     .build();
/// ```
#[derive(Debug, Default)]
#[must_use = "builders do nothing until .build() is called"]
pub struct McpSchemaBuilder {
    name: String,
    tools: Vec<McpTool>,
}

impl McpSchemaBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: Vec::new(),
        }
    }

    pub fn add_tool(mut self, tool: McpTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn build(self) -> McpSchema {
        McpSchema {
            name: self.name,
            tools: self.tools,
        }
    }
}

/// Builder for creating MCP tools (instructions).
///
/// # Example
///
/// ```
/// use mcpsol_core::{McpToolBuilder, ArgType};
///
/// let tool = McpToolBuilder::new("transfer")
///     .description("Transfer tokens between accounts")
///     .signer_writable("from")
///     .writable("to")
///     .arg("amount", ArgType::U64)
///     .build();
/// ```
#[derive(Debug, Default)]
#[must_use = "builders do nothing until .build() is called"]
pub struct McpToolBuilder {
    name: String,
    description: Option<String>,
    accounts: Vec<McpAccountMeta>,
    args: Vec<McpArg>,
}

impl McpToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            accounts: Vec::new(),
            args: Vec::new(),
        }
    }

    /// Add a description for AI agents to understand the tool
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn account(mut self, name: impl Into<String>, is_signer: bool, is_writable: bool) -> Self {
        self.accounts.push(McpAccountMeta {
            name: name.into(),
            description: None,
            is_signer,
            is_writable,
        });
        self
    }

    /// Add an account with a description for AI agents
    pub fn account_with_desc(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        is_signer: bool,
        is_writable: bool,
    ) -> Self {
        self.accounts.push(McpAccountMeta {
            name: name.into(),
            description: Some(desc.into()),
            is_signer,
            is_writable,
        });
        self
    }

    pub fn signer(self, name: impl Into<String>) -> Self {
        self.account(name, true, false)
    }

    /// Add a signer account with description
    pub fn signer_desc(self, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.account_with_desc(name, desc, true, false)
    }

    pub fn writable(self, name: impl Into<String>) -> Self {
        self.account(name, false, true)
    }

    /// Add a writable account with description
    pub fn writable_desc(self, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.account_with_desc(name, desc, false, true)
    }

    pub fn signer_writable(self, name: impl Into<String>) -> Self {
        self.account(name, true, true)
    }

    /// Add a signer+writable account with description
    pub fn signer_writable_desc(self, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.account_with_desc(name, desc, true, true)
    }

    pub fn arg(mut self, name: impl Into<String>, arg_type: ArgType) -> Self {
        self.args.push(McpArg {
            name: name.into(),
            description: None,
            arg_type,
        });
        self
    }

    /// Add an argument with a description for AI agents
    pub fn arg_desc(mut self, name: impl Into<String>, desc: impl Into<String>, arg_type: ArgType) -> Self {
        self.args.push(McpArg {
            name: name.into(),
            description: Some(desc.into()),
            arg_type,
        });
        self
    }

    pub fn build(self) -> McpTool {
        use crate::instruction_discriminator;
        McpTool {
            discriminator: instruction_discriminator(&self.name),
            name: self.name,
            description: self.description,
            accounts: self.accounts,
            args: self.args,
        }
    }
}

// ============================================================================
// CachedSchemaPages - Pre-computed paginated schema for CU optimization
// ============================================================================

/// Pre-computed paginated schema pages for CU-efficient `list_tools` responses.
///
/// This struct caches the serialized JSON bytes for each pagination page,
/// avoiding repeated serialization overhead on subsequent `list_tools` calls.
///
/// # Example
///
/// ```ignore
/// use mcpsol_core::CachedSchemaPages;
///
/// fn build_schema() -> McpSchema {
///     // ... build your schema
/// }
///
/// static CACHED: std::sync::OnceLock<CachedSchemaPages> = std::sync::OnceLock::new();
///
/// fn get_page(cursor: u8) -> &'static [u8] {
///     CACHED.get_or_init(|| CachedSchemaPages::from_schema(build_schema()))
///         .get_page(cursor)
/// }
/// ```
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct CachedSchemaPages {
    /// Pre-serialized JSON bytes for each page
    pages: Vec<Vec<u8>>,
}

#[cfg(feature = "std")]
impl CachedSchemaPages {
    /// Create cached pages from a schema.
    ///
    /// This pre-computes and caches the serialized JSON for each pagination page.
    /// The first page (cursor=0) contains the first tool, and so on.
    pub fn from_schema(schema: McpSchema) -> Self {
        use crate::generate_paginated_schema_bytes;

        let num_pages = schema.tools.len().max(1);
        let mut pages = Vec::with_capacity(num_pages);

        for cursor in 0..num_pages {
            let page_bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
            pages.push(page_bytes);
        }

        Self { pages }
    }

    /// Get a cached page by cursor index.
    ///
    /// Returns an empty slice if cursor is out of bounds.
    /// This is a zero-allocation operation after initialization.
    #[inline]
    pub fn get_page(&self, cursor: u8) -> &[u8] {
        self.pages
            .get(cursor as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the number of pages (tools) in this cached schema.
    #[inline]
    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let schema = McpSchemaBuilder::new("test_program")
            .add_tool(
                McpToolBuilder::new("transfer")
                    .signer_writable("from")
                    .writable("to")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .build();

        assert_eq!(schema.name, "test_program");
        assert_eq!(schema.tools.len(), 1);
        assert_eq!(schema.tools[0].name, "transfer");
        assert_eq!(schema.tools[0].accounts.len(), 2);
        assert_eq!(schema.tools[0].args.len(), 1);
    }
}
