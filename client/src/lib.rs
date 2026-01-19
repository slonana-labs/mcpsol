// Large error variant is from solana_client external crate - boxing would break From impl
#![allow(clippy::result_large_err, clippy::large_enum_variant)]

//! MCP Client Library for Solana
//!
//! Discover and interact with MCP-enabled Solana programs.
//!
//! Supports both compact and paginated verbose schema formats:
//! - **Compact**: All tools in one response (abbreviated keys)
//! - **Paginated**: One tool per page with full descriptions
//!
//! # Example
//!
//! ```rust,ignore
//! use mcpsol_client::McpClient;
//! use solana_sdk::pubkey::Pubkey;
//!
//! let client = McpClient::new("https://api.devnet.solana.com");
//! let program_id: Pubkey = "YourProgram111111111111111111111111111111111".parse()?;
//!
//! // Discover available tools (auto-detects schema format)
//! let schema = client.list_tools(&program_id)?;
//! println!("Program: {}", schema.name);
//! for tool in &schema.tools {
//!     println!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or(""));
//! }
//!
//! // For paginated schemas with full descriptions, fetch all pages:
//! let schema = client.list_tools_full(&program_id)?;
//!
//! // Build and send an instruction
//! let ix = client.build_instruction(
//!     &program_id,
//!     "increment",
//!     &[("counter", counter_pubkey), ("authority", authority_pubkey)],
//!     &[("amount", "100")],
//!     &schema,
//! )?;
//! ```

use mcpsol_core::LIST_TOOLS_DISCRIMINATOR;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur when interacting with MCP programs.
#[derive(Error, Debug)]
pub enum McpClientError {
    /// RPC communication error
    #[error("RPC error: {0}")]
    Rpc(#[from] solana_client::client_error::ClientError),

    #[error("Failed to parse schema: {0}")]
    ParseSchema(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Missing required parameter: {0}")]
    MissingParam(String),

    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Invalid argument value: {0}")]
    InvalidArg(String),

    #[error("No return data from program")]
    NoReturnData,
}

pub type Result<T> = std::result::Result<T, McpClientError>;

/// Parsed MCP schema from on-chain program.
///
/// Supports both compact and verbose schema formats.
#[derive(Debug, Clone, Deserialize)]
pub struct ParsedSchema {
    #[serde(rename = "v")]
    pub version: String,
    pub name: String,
    pub tools: Vec<ParsedTool>,
    /// Pagination cursor for verbose format (None = last page or compact format)
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Parsed tool from MCP schema.
///
/// Supports both compact format (abbreviated keys) and verbose format (full keys).
#[derive(Debug, Clone, Deserialize)]
pub struct ParsedTool {
    /// Tool name - supports both "n" (compact) and "name" (verbose)
    #[serde(alias = "n")]
    pub name: String,
    /// Tool description - supports both "i" (compact) and "description" (verbose)
    #[serde(alias = "i")]
    pub description: Option<String>,
    /// Discriminator hex - supports both "d" (compact) and "discriminator" (verbose)
    #[serde(alias = "d")]
    pub discriminator: String,
    /// Parameters - supports "p" (compact), "parameters" (verbose), and "params"
    #[serde(alias = "p", alias = "parameters", default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    /// Required parameters (compact format only)
    #[serde(alias = "r", default)]
    pub required: Vec<String>,
}

impl ParsedTool {
    /// Get the discriminator as bytes
    ///
    /// # Returns
    /// - `Ok([u8; 8])` - The decoded discriminator
    /// - `Err` - If the discriminator is invalid hex or too short
    pub fn discriminator_bytes(&self) -> Result<[u8; 8]> {
        let decoded = hex::decode(&self.discriminator)
            .map_err(|_| McpClientError::ParseSchema(
                format!("Invalid discriminator hex: {}", self.discriminator)
            ))?;

        if decoded.len() < 8 {
            return Err(McpClientError::ParseSchema(
                format!("Discriminator too short: {} bytes", decoded.len())
            ));
        }

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&decoded[..8]);
        Ok(bytes)
    }

    /// Check if a parameter is an account (pubkey type).
    ///
    /// Supports both compact format (value is "pubkey" string) and
    /// verbose format (object with "type": "pubkey").
    pub fn is_account(&self, name: &str) -> bool {
        self.params.get(name)
            .map(|v| {
                // Compact format: "pubkey"
                if v.as_str() == Some("pubkey") {
                    return true;
                }
                // Verbose format: {"type": "pubkey", ...}
                if let Some(obj) = v.as_object() {
                    return obj.get("type").and_then(|t| t.as_str()) == Some("pubkey");
                }
                false
            })
            .unwrap_or(false)
    }

    /// Check if an account is a signer.
    ///
    /// Supports both compact format (name suffix `_s` or `_sw`) and
    /// verbose format (object with `"signer": true`).
    pub fn is_signer(&self, name: &str) -> bool {
        // Compact format: suffix
        if name.ends_with("_s") || name.ends_with("_sw") {
            return true;
        }
        // Verbose format: nested object
        self.params.get(name)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("signer"))
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
    }

    /// Check if an account is writable.
    ///
    /// Supports both compact format (name suffix `_w` or `_sw`) and
    /// verbose format (object with `"writable": true`).
    pub fn is_writable(&self, name: &str) -> bool {
        // Compact format: suffix
        if name.ends_with("_w") || name.ends_with("_sw") {
            return true;
        }
        // Verbose format: nested object
        self.params.get(name)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("writable"))
            .and_then(|w| w.as_bool())
            .unwrap_or(false)
    }

    /// Get the parameter type as a string.
    ///
    /// Supports both compact format (value is type string) and
    /// verbose format (object with "type" field).
    pub fn get_param_type(&self, name: &str) -> Option<&str> {
        self.params.get(name).and_then(|v| {
            // Compact format: direct string
            if let Some(s) = v.as_str() {
                return Some(s);
            }
            // Verbose format: nested object
            v.as_object()
                .and_then(|obj| obj.get("type"))
                .and_then(|t| t.as_str())
        })
    }

    /// Get parameter description (verbose format only).
    pub fn get_param_description(&self, name: &str) -> Option<&str> {
        self.params.get(name)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("description"))
            .and_then(|d| d.as_str())
    }

    /// Get the base name without suffix.
    ///
    /// For compact format, strips `_s`, `_w`, `_sw` suffixes.
    /// Must check `_sw` first since `trim_end_matches` is greedy.
    pub fn base_name(name: &str) -> &str {
        name.trim_end_matches("_sw")
            .trim_end_matches("_s")
            .trim_end_matches("_w")
    }

    /// Get all parameter names (for building required list from verbose format).
    pub fn param_names(&self) -> Vec<&String> {
        self.params.keys().collect()
    }

    /// Get required parameters.
    ///
    /// For compact format, uses the `required` field.
    /// For verbose format, all parameters in `params` are considered required.
    pub fn required_params(&self) -> Vec<&str> {
        if !self.required.is_empty() {
            // Compact format
            self.required.iter().map(|s| s.as_str()).collect()
        } else {
            // Verbose format - all params are required
            self.params.keys().map(|s| s.as_str()).collect()
        }
    }
}

/// MCP Client for discovering and calling Solana programs
pub struct McpClient {
    rpc: RpcClient,
}

impl McpClient {
    /// Create a new client
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    /// Create from existing RpcClient
    pub const fn from_rpc(rpc: RpcClient) -> Self {
        Self { rpc }
    }

    /// Discover available tools by calling list_tools (first page only).
    ///
    /// For paginated schemas, use [`list_tools_full`] to fetch all pages.
    pub fn list_tools(&self, program_id: &Pubkey) -> Result<ParsedSchema> {
        self.list_tools_page(program_id, 0)
    }

    /// Fetch a specific page of the schema.
    ///
    /// The cursor is the page number (0-indexed).
    pub fn list_tools_page(&self, program_id: &Pubkey, cursor: u8) -> Result<ParsedSchema> {
        // Build list_tools instruction with optional cursor
        let mut data = LIST_TOOLS_DISCRIMINATOR.to_vec();
        if cursor > 0 {
            data.push(cursor);
        }

        let ix = Instruction {
            program_id: *program_id,
            accounts: vec![],
            data,
        };

        // Simulate transaction
        let payer = Keypair::new();
        let blockhash = self.rpc.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        let result = self.rpc.simulate_transaction(&tx)?;

        // Extract return data
        let return_data = result.value.return_data
            .ok_or(McpClientError::NoReturnData)?;

        // Decode base64 return data
        let schema_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &return_data.data.0,
        ).map_err(|e| McpClientError::ParseSchema(e.to_string()))?;

        // Parse JSON schema
        let schema: ParsedSchema = serde_json::from_slice(&schema_bytes)
            .map_err(|e| McpClientError::ParseSchema(e.to_string()))?;

        Ok(schema)
    }

    /// Fetch all pages of a paginated schema.
    ///
    /// For non-paginated (compact) schemas, returns the single page.
    /// For paginated schemas, fetches all pages and combines tools.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let schema = client.list_tools_full(&program_id)?;
    /// // schema.tools contains all tools from all pages
    /// ```
    pub fn list_tools_full(&self, program_id: &Pubkey) -> Result<ParsedSchema> {
        let mut schema = self.list_tools_page(program_id, 0)?;
        let mut cursor = 1u8;

        // Keep fetching while there's a next cursor
        while schema.next_cursor.is_some() {
            let next_page = self.list_tools_page(program_id, cursor)?;
            schema.tools.extend(next_page.tools);
            schema.next_cursor = next_page.next_cursor;
            cursor = cursor.saturating_add(1);

            // Safety limit to prevent infinite loops
            if cursor > 100 {
                break;
            }
        }

        Ok(schema)
    }

    /// Build an instruction from tool name and parameters.
    ///
    /// Supports both compact and verbose schema formats.
    pub fn build_instruction(
        &self,
        program_id: &Pubkey,
        tool_name: &str,
        accounts: &[(&str, Pubkey)],
        args: &[(&str, &str)],
        schema: &ParsedSchema,
    ) -> Result<Instruction> {
        // Find tool
        let tool = schema.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| McpClientError::ToolNotFound(tool_name.to_string()))?;

        // Get required parameters (works for both formats)
        let required_params = tool.required_params();

        // Build account metas
        let mut account_metas = Vec::new();
        for required in &required_params {
            if !tool.is_account(required) {
                continue; // Skip non-account params
            }

            let base = ParsedTool::base_name(required);
            let pubkey = accounts.iter()
                .find(|(name, _)| *name == base || *name == *required)
                .map(|(_, pk)| *pk)
                .ok_or_else(|| McpClientError::MissingParam((*required).to_string()))?;

            account_metas.push(AccountMeta {
                pubkey,
                is_signer: tool.is_signer(required),
                is_writable: tool.is_writable(required),
            });
        }

        // Build instruction data
        let mut data = tool.discriminator_bytes()?.to_vec();

        // Add args in order
        for required in &required_params {
            if tool.is_account(required) {
                continue; // Skip account params
            }

            let arg_type = tool.get_param_type(required).unwrap_or("str");

            let value = args.iter()
                .find(|(name, _)| *name == *required)
                .map(|(_, v)| *v)
                .ok_or_else(|| McpClientError::MissingParam((*required).to_string()))?;

            // Serialize arg based on type
            // Note: compact schema uses "int" for integers, "bool" for booleans, "str" for strings
            match arg_type {
                // Compact schema type - default to u64 for integers
                "int" => {
                    let v: u64 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "u8" => {
                    let v: u8 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "u16" => {
                    let v: u16 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "u32" => {
                    let v: u32 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "u64" => {
                    let v: u64 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "u128" => {
                    let v: u128 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "i8" => {
                    let v: i8 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "i16" => {
                    let v: i16 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "i32" => {
                    let v: i32 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "i64" => {
                    let v: i64 = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&v.to_le_bytes());
                }
                "bool" => {
                    let v: bool = value.parse()
                        .map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.push(if v { 1 } else { 0 });
                }
                "pubkey" => {
                    let pk = Pubkey::from_str(value)
                        .map_err(|_| McpClientError::InvalidPubkey((*required).to_string()))?;
                    data.extend_from_slice(pk.as_ref());
                }
                "str" => {
                    // Borsh string: 4-byte length + bytes
                    let bytes = value.as_bytes();
                    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    data.extend_from_slice(bytes);
                }
                "bytes" => {
                    // Base64 encoded bytes
                    let decoded = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        value,
                    ).map_err(|_| McpClientError::InvalidArg((*required).to_string()))?;
                    data.extend_from_slice(&(decoded.len() as u32).to_le_bytes());
                    data.extend_from_slice(&decoded);
                }
                _ => {
                    // Unknown type, try as string
                    let bytes = value.as_bytes();
                    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    data.extend_from_slice(bytes);
                }
            }
        }

        Ok(Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data,
        })
    }

    /// Get the underlying RPC client
    pub const fn rpc(&self) -> &RpcClient {
        &self.rpc
    }
}

/// Helper to decode hex strings
mod hex {
    pub fn decode(s: &str) -> std::result::Result<Vec<u8>, ()> {
        if !s.len().is_multiple_of(2) {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema() {
        let json = r#"{
            "v": "2024-11-05",
            "name": "counter",
            "tools": [
                {
                    "n": "increment",
                    "i": "Add to counter",
                    "d": "0b12680968ae3b21",
                    "p": {"counter_w": "pubkey", "authority_s": "pubkey", "amount": "u64"},
                    "r": ["counter_w", "authority_s", "amount"]
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.name, "counter");
        assert_eq!(schema.tools.len(), 1);

        let tool = &schema.tools[0];
        assert_eq!(tool.name, "increment");
        assert!(tool.is_writable("counter_w"));
        assert!(tool.is_signer("authority_s"));
        assert!(!tool.is_account("amount"));
    }

    #[test]
    fn test_discriminator_parse() {
        let tool = ParsedTool {
            name: "test".to_string(),
            description: None,
            discriminator: "0b12680968ae3b21".to_string(),
            params: serde_json::Map::new(),
            required: vec![],
        };

        let bytes = tool.discriminator_bytes().unwrap();
        assert_eq!(bytes, [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21]);
    }

    #[test]
    fn test_discriminator_invalid_hex() {
        let tool = ParsedTool {
            name: "test".to_string(),
            description: None,
            discriminator: "invalid_hex".to_string(),
            params: serde_json::Map::new(),
            required: vec![],
        };

        assert!(tool.discriminator_bytes().is_err());
    }

    #[test]
    fn test_discriminator_too_short() {
        let tool = ParsedTool {
            name: "test".to_string(),
            description: None,
            discriminator: "0b1268".to_string(), // Only 3 bytes
            params: serde_json::Map::new(),
            required: vec![],
        };

        assert!(tool.discriminator_bytes().is_err());
    }

    #[test]
    fn test_base_name() {
        assert_eq!(ParsedTool::base_name("counter_w"), "counter");
        assert_eq!(ParsedTool::base_name("authority_s"), "authority");
        assert_eq!(ParsedTool::base_name("payer_sw"), "payer");
        assert_eq!(ParsedTool::base_name("amount"), "amount");
    }

    #[test]
    fn test_parse_list_tools_no_params() {
        // list_tools has no params or required fields - must parse with defaults
        let json = r#"{
            "v": "2024-11-05",
            "name": "counter",
            "tools": [
                {"n": "list_tools", "d": "42195e6a55fd41c0"}
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.tools.len(), 1);
        assert_eq!(schema.tools[0].name, "list_tools");
        assert!(schema.tools[0].params.is_empty());
        assert!(schema.tools[0].required.is_empty());
    }

    #[test]
    fn test_int_type_parsing() {
        // Compact schema uses "int" for integers
        let json = r#"{
            "v": "2024-11-05",
            "name": "counter",
            "tools": [
                {
                    "n": "increment",
                    "d": "0b12680968ae3b21",
                    "p": {"counter_w": "pubkey", "amount": "int"},
                    "r": ["counter_w", "amount"]
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        let tool = &schema.tools[0];
        assert_eq!(tool.params.get("amount").unwrap().as_str().unwrap(), "int");
    }

    // ========================================================================
    // Verbose (Paginated) Schema Format Tests
    // ========================================================================

    #[test]
    fn test_parse_verbose_schema() {
        // Verbose schema format with full keys and nested objects
        let json = r#"{
            "v": "2024-11-05",
            "name": "counter",
            "tools": [
                {
                    "name": "increment",
                    "description": "Add amount to counter value",
                    "discriminator": "0b12680968ae3b21",
                    "parameters": {
                        "counter": {"type": "pubkey", "writable": true, "description": "The counter to modify"},
                        "authority": {"type": "pubkey", "signer": true, "description": "Must match counter authority"},
                        "amount": {"type": "u64", "description": "Value to add"}
                    }
                }
            ],
            "nextCursor": "1"
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.name, "counter");
        assert_eq!(schema.next_cursor, Some("1".to_string()));
        assert_eq!(schema.tools.len(), 1);

        let tool = &schema.tools[0];
        assert_eq!(tool.name, "increment");
        assert_eq!(tool.description, Some("Add amount to counter value".to_string()));

        // Check verbose format parsing
        assert!(tool.is_account("counter"));
        assert!(tool.is_account("authority"));
        assert!(!tool.is_account("amount"));

        assert!(tool.is_writable("counter"));
        assert!(!tool.is_writable("authority"));

        assert!(tool.is_signer("authority"));
        assert!(!tool.is_signer("counter"));

        // Check get_param_type
        assert_eq!(tool.get_param_type("counter"), Some("pubkey"));
        assert_eq!(tool.get_param_type("amount"), Some("u64"));

        // Check get_param_description
        assert_eq!(tool.get_param_description("counter"), Some("The counter to modify"));
        assert_eq!(tool.get_param_description("amount"), Some("Value to add"));
    }

    #[test]
    fn test_parse_verbose_last_page() {
        // Last page has no nextCursor
        let json = r#"{
            "v": "2024-11-05",
            "name": "counter",
            "tools": [
                {
                    "name": "decrement",
                    "description": "Subtract from counter",
                    "discriminator": "6ae3a83bf81b9665",
                    "parameters": {
                        "counter": {"type": "pubkey", "writable": true},
                        "amount": {"type": "u64"}
                    }
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.next_cursor, None);
    }

    #[test]
    fn test_required_params_verbose() {
        // Verbose format: all params are required
        let json = r#"{
            "v": "2024-11-05",
            "name": "test",
            "tools": [
                {
                    "name": "transfer",
                    "discriminator": "0b12680968ae3b21",
                    "parameters": {
                        "from": {"type": "pubkey", "signer": true, "writable": true},
                        "to": {"type": "pubkey", "writable": true},
                        "amount": {"type": "u64"}
                    }
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        let tool = &schema.tools[0];

        let required = tool.required_params();
        assert_eq!(required.len(), 3);
        assert!(required.contains(&"from"));
        assert!(required.contains(&"to"));
        assert!(required.contains(&"amount"));
    }

    #[test]
    fn test_required_params_compact() {
        // Compact format: uses explicit required array
        let json = r#"{
            "v": "2024-11-05",
            "name": "test",
            "tools": [
                {
                    "n": "transfer",
                    "d": "0b12680968ae3b21",
                    "p": {"from_sw": "pubkey", "to_w": "pubkey", "amount": "u64"},
                    "r": ["from_sw", "to_w", "amount"]
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        let tool = &schema.tools[0];

        let required = tool.required_params();
        assert_eq!(required.len(), 3);
        assert!(required.contains(&"from_sw"));
        assert!(required.contains(&"to_w"));
        assert!(required.contains(&"amount"));
    }

    #[test]
    fn test_mixed_format_compatibility() {
        // Test that both name/n and description/i work
        let json = r#"{
            "v": "2024-11-05",
            "name": "test",
            "tools": [
                {
                    "n": "test1",
                    "i": "Compact description",
                    "d": "0b12680968ae3b21"
                },
                {
                    "name": "test2",
                    "description": "Verbose description",
                    "discriminator": "0b12680968ae3b22"
                }
            ]
        }"#;

        let schema: ParsedSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.tools[0].name, "test1");
        assert_eq!(schema.tools[0].description, Some("Compact description".to_string()));
        assert_eq!(schema.tools[1].name, "test2");
        assert_eq!(schema.tools[1].description, Some("Verbose description".to_string()));
    }
}
