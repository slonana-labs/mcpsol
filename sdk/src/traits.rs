use crate::mcp::McpSchema;

/// Trait for programs that expose MCP interface
pub trait McpProgram {
    /// Program name for MCP
    const NAME: &'static str;

    /// Program description for MCP
    const DESCRIPTION: &'static str;

    /// Generate MCP schema for this program
    fn mcp_schema() -> McpSchema;
}

/// Trait for instructions that expose MCP tool interface
pub trait McpInstruction {
    /// Instruction discriminator (first 8 bytes of instruction data)
    const DISCRIMINATOR: [u8; 8];

    /// Tool name for MCP
    const TOOL_NAME: &'static str;

    /// Tool description for MCP
    const TOOL_DESCRIPTION: &'static str;

    /// Generate MCP tool schema
    fn mcp_tool_schema() -> crate::mcp::McpTool;
}

/// Trait for account types that expose MCP resource interface
pub trait McpResource {
    /// Resource URI pattern (e.g., "solana://mainnet/{address}")
    const URI_PATTERN: &'static str;

    /// Resource name for MCP
    const RESOURCE_NAME: &'static str;

    /// Resource description for MCP
    const RESOURCE_DESCRIPTION: &'static str;

    /// Generate MCP resource schema
    fn mcp_resource_schema() -> crate::mcp::McpResourceDef;
}
