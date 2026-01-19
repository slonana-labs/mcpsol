//! mcpsol-native: MCP schema generation for native Solana programs
//!
//! Add MCP tool discovery to native solana-program based programs.
//!
//! # Usage
//!
//! ```rust,ignore
//! use solana_program::{
//!     account_info::AccountInfo,
//!     entrypoint,
//!     entrypoint::ProgramResult,
//!     pubkey::Pubkey,
//! };
//! use mcpsol_native::prelude::*;
//!
//! // Define your schema
//! const SCHEMA: &[u8] = mcp_schema_bytes!(
//!     name: "my_program",
//!     tools: [
//!         tool("transfer")
//!             .signer_writable("from")
//!             .writable("to")
//!             .arg("amount", ArgType::U64),
//!     ]
//! );
//!
//! entrypoint!(process_instruction);
//!
//! pub fn process_instruction(
//!     program_id: &Pubkey,
//!     accounts: &[AccountInfo],
//!     data: &[u8],
//! ) -> ProgramResult {
//!     // Check for list_tools
//!     if data.len() >= 8 && data[..8] == LIST_TOOLS_DISCRIMINATOR {
//!         return list_tools(SCHEMA);
//!     }
//!
//!     // Handle other instructions...
//!     Ok(())
//! }
//! ```

pub use mcpsol_core::{
    // Constants
    PROTOCOL_VERSION,
    LIST_TOOLS_DISCRIMINATOR,
    MAX_RETURN_DATA_SIZE,
    // Discriminator functions
    instruction_discriminator,
    account_discriminator,
    // Schema types
    McpSchema,
    McpTool,
    McpAccountMeta,
    McpArg,
    ArgType,
    // Builders
    McpSchemaBuilder,
    McpToolBuilder,
    // JSON generation - compact (all tools, abbreviated keys)
    generate_compact_schema,
    generate_schema_bytes,
    estimate_schema_size,
    // JSON generation - paginated (one tool per page, full descriptions)
    generate_paginated_schema,
    generate_paginated_schema_bytes,
};

use solana_program::{entrypoint::ProgramResult, program::set_return_data};

/// Handle list_tools instruction by returning schema via set_return_data
/// Use this for compact schema (all tools in one response)
pub fn list_tools(schema_bytes: &[u8]) -> ProgramResult {
    set_return_data(schema_bytes);
    Ok(())
}

/// Handle paginated list_tools with full descriptions
///
/// Extracts cursor from instruction data (byte 8) and returns one tool per page.
/// Use this when you need full parameter descriptions for AI agents.
///
/// # Arguments
/// * `schema` - The full MCP schema
/// * `data` - Instruction data: [discriminator: 8 bytes][cursor: 1 byte (optional)]
///
/// # Example
/// ```ignore
/// match discriminator {
///     LIST_TOOLS => list_tools_paginated(&schema, data),
///     // ...
/// }
/// ```
pub fn list_tools_paginated(schema: &McpSchema, data: &[u8]) -> ProgramResult {
    // Cursor is optional byte after discriminator (default 0)
    let cursor = data.get(8).copied().unwrap_or(0);
    let schema_bytes = generate_paginated_schema_bytes(schema, cursor);
    set_return_data(&schema_bytes);
    Ok(())
}

/// Check if instruction data matches list_tools discriminator
#[inline]
pub fn is_list_tools(data: &[u8]) -> bool {
    data.len() >= 8 && data[..8] == LIST_TOOLS_DISCRIMINATOR
}

/// Extract cursor from list_tools instruction data
/// Returns 0 if cursor byte not present
#[inline]
pub fn get_list_tools_cursor(data: &[u8]) -> u8 {
    data.get(8).copied().unwrap_or(0)
}

/// Trait for programs that expose MCP schemas
pub trait McpProgram {
    /// Get the MCP schema for this program
    fn mcp_schema() -> McpSchema;

    /// Get the schema as JSON bytes
    fn schema_bytes() -> Vec<u8> {
        generate_schema_bytes(&Self::mcp_schema())
    }
}

/// Helper function to create a tool builder
pub fn tool(name: &str) -> McpToolBuilder {
    McpToolBuilder::new(name)
}

/// Macro for checking discriminator match
#[macro_export]
macro_rules! match_discriminator {
    ($data:expr, $name:literal) => {
        $data.len() >= 8 && $data[..8] == $crate::instruction_discriminator($name)
    };
}

/// Macro for generating discriminator constant at compile time
#[macro_export]
macro_rules! discriminator {
    ($name:literal) => {{
        // Note: This is computed at runtime, use the macros crate for compile-time
        $crate::instruction_discriminator($name)
    }};
}

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        // Core re-exports
        PROTOCOL_VERSION,
        LIST_TOOLS_DISCRIMINATOR,
        MAX_RETURN_DATA_SIZE,
        McpSchema,
        McpTool,
        McpAccountMeta,
        McpArg,
        ArgType,
        McpSchemaBuilder,
        McpToolBuilder,
        // Compact schema (all tools in one response)
        generate_compact_schema,
        generate_schema_bytes,
        estimate_schema_size,
        // Paginated schema (one tool per page, full descriptions)
        generate_paginated_schema,
        generate_paginated_schema_bytes,
        instruction_discriminator,
        account_discriminator,
        // Native-specific
        list_tools,
        list_tools_paginated,
        is_list_tools,
        get_list_tools_cursor,
        McpProgram,
        tool,
        match_discriminator,
        discriminator,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_list_tools() {
        // Valid list_tools discriminator
        let valid = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0];
        assert!(is_list_tools(&valid));

        // With extra data
        let with_extra = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0, 0x00, 0x01];
        assert!(is_list_tools(&with_extra));

        // Wrong discriminator
        let wrong = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(!is_list_tools(&wrong));

        // Too short
        let short = [0x42, 0x19, 0x5e];
        assert!(!is_list_tools(&short));
    }

    #[test]
    fn test_match_discriminator_macro() {
        let data = instruction_discriminator("transfer");
        assert!(match_discriminator!(&data, "transfer"));
        assert!(!match_discriminator!(&data, "other"));
    }

    #[test]
    fn test_tool_builder() {
        let tool = tool("increment")
            .writable("counter")
            .signer("authority")
            .arg("amount", ArgType::U64)
            .build();

        assert_eq!(tool.name, "increment");
        assert_eq!(tool.accounts.len(), 2);
        assert_eq!(tool.args.len(), 1);
    }
}
