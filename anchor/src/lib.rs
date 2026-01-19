//! mcpsol-anchor: MCP schema generation for Anchor programs
//!
//! Add MCP tool discovery to your Anchor programs with minimal changes.
//!
//! # Usage
//!
//! ```rust,ignore
//! use anchor_lang::prelude::*;
//! use mcpsol_anchor::prelude::*;
//!
//! // Add #[mcp_tool] to instructions you want exposed
//! #[program]
//! pub mod my_program {
//!     use super::*;
//!
//!     pub fn list_tools(ctx: Context<ListTools>) -> Result<()> {
//!         ctx.accounts.return_schema::<MyProgram>()
//!     }
//!
//!     #[mcp_tool]
//!     pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
//!         // your implementation
//!     }
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
    // JSON generation
    generate_compact_schema,
    generate_schema_bytes,
};

use anchor_lang::prelude::*;

/// Trait for programs that expose MCP schemas
pub trait McpProgram {
    /// Get the MCP schema for this program
    fn mcp_schema() -> McpSchema;

    /// Get the schema as JSON bytes (for set_return_data)
    fn schema_bytes() -> Vec<u8> {
        generate_schema_bytes(&Self::mcp_schema())
    }
}

/// Empty accounts context for list_tools
/// No accounts needed - just returns schema via return_data
#[derive(Accounts)]
pub struct ListTools {}

impl ListTools {
    /// Return the MCP schema via set_return_data
    pub fn return_schema<P: McpProgram>(&self) -> Result<()> {
        let schema_bytes = P::schema_bytes();
        anchor_lang::solana_program::program::set_return_data(&schema_bytes);
        Ok(())
    }
}

/// Convenience macro for defining MCP schema inline
///
/// # Example
///
/// ```rust,ignore
/// mcp_schema!(MyProgram {
///     name: "my_program",
///     tools: [
///         tool("transfer")
///             .signer_writable("from")
///             .writable("to")
///             .arg("amount", ArgType::U64),
///         tool("initialize")
///             .signer("authority")
///             .writable("account"),
///     ]
/// });
/// ```
#[macro_export]
macro_rules! mcp_schema {
    ($program:ident {
        name: $name:literal,
        tools: [$($tool:expr),* $(,)?]
    }) => {
        impl $crate::McpProgram for $program {
            fn mcp_schema() -> $crate::McpSchema {
                $crate::McpSchemaBuilder::new($name)
                    $(.add_tool($tool.build()))*
                    .build()
            }
        }
    };
}

/// Helper function to create a tool builder
pub fn tool(name: &str) -> McpToolBuilder {
    McpToolBuilder::new(name)
}

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        // Core re-exports
        PROTOCOL_VERSION,
        LIST_TOOLS_DISCRIMINATOR,
        McpSchema,
        McpTool,
        McpAccountMeta,
        McpArg,
        ArgType,
        McpSchemaBuilder,
        McpToolBuilder,
        generate_compact_schema,
        // Anchor-specific
        McpProgram,
        ListTools,
        tool,
        mcp_schema,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProgram;

    impl McpProgram for TestProgram {
        fn mcp_schema() -> McpSchema {
            McpSchemaBuilder::new("test_program")
                .add_tool(
                    McpToolBuilder::new("list_tools")
                        .build()
                )
                .add_tool(
                    McpToolBuilder::new("transfer")
                        .signer_writable("from")
                        .writable("to")
                        .arg("amount", ArgType::U64)
                        .build()
                )
                .build()
        }
    }

    #[test]
    fn test_schema_generation() {
        let schema = TestProgram::mcp_schema();
        assert_eq!(schema.name, "test_program");
        assert_eq!(schema.tools.len(), 2);
    }

    #[test]
    fn test_schema_bytes() {
        let bytes = TestProgram::schema_bytes();
        let json = String::from_utf8(bytes).unwrap();
        assert!(json.contains("\"name\":\"test_program\""));
        assert!(json.contains("\"n\":\"transfer\""));
    }
}
