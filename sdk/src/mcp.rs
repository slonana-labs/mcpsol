//! MCP Schema types for Solana programs
//!
//! These types map directly to the Model Context Protocol specification.
//! Instead of generating IDL, programs generate MCP schemas that AI agents
//! can use to understand and interact with the program.

use serde::{Deserialize, Serialize};

/// Complete MCP schema for a Solana program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchema {
    /// Protocol version
    pub protocol_version: &'static str,
    /// Program metadata
    pub program: ProgramMeta,
    /// Available tools (instructions)
    pub tools: Vec<McpTool>,
    /// Available resources (account types)
    pub resources: Vec<McpResourceDef>,
}

impl Default for McpSchema {
    fn default() -> Self {
        Self {
            protocol_version: "2024-11-05",
            program: ProgramMeta::default(),
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }
}

/// Program metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgramMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub program_id: Option<String>,
}

/// MCP Tool definition (maps to a Solana instruction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (instruction name)
    pub name: String,
    /// Human-readable description for AI
    pub description: String,
    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: InputSchema,
}

/// JSON Schema for tool inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl Default for InputSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: serde_json::Map::new(),
            required: Vec::new(),
        }
    }
}

/// Property definition for tool inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDef {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// MCP Resource definition (maps to a Solana account type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceDef {
    /// URI template for accessing this resource
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// MIME type of resource content
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Schema for the account data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Account parameter for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountParam {
    /// Account name
    pub name: String,
    /// Description of this account's purpose
    pub description: String,
    /// Whether this account must sign
    pub is_signer: bool,
    /// Whether this account is mutable
    pub is_writable: bool,
    /// Optional: expected account type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    /// Optional: PDA seeds description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_seeds: Option<Vec<String>>,
}

/// Builder for creating MCP tool schemas
pub struct McpToolBuilder {
    name: String,
    description: String,
    properties: serde_json::Map<String, serde_json::Value>,
    required: Vec<String>,
}

impl McpToolBuilder {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            properties: serde_json::Map::new(),
            required: Vec::new(),
        }
    }

    /// Add an account parameter
    pub fn account(mut self, param: AccountParam) -> Self {
        let mut props = serde_json::Map::new();
        props.insert("type".to_string(), serde_json::json!("string"));
        props.insert("description".to_string(), serde_json::json!(param.description));
        props.insert("format".to_string(), serde_json::json!("solana-pubkey"));

        // Add metadata as x- extensions
        props.insert("x-is-signer".to_string(), serde_json::json!(param.is_signer));
        props.insert("x-is-writable".to_string(), serde_json::json!(param.is_writable));

        if let Some(ref account_type) = param.account_type {
            props.insert("x-account-type".to_string(), serde_json::json!(account_type));
        }
        if let Some(ref seeds) = param.pda_seeds {
            props.insert("x-pda-seeds".to_string(), serde_json::json!(seeds));
        }

        self.properties.insert(param.name.clone(), serde_json::Value::Object(props));
        self.required.push(param.name);
        self
    }

    /// Add a u64 argument
    pub fn arg_u64(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let props = serde_json::json!({
            "type": "integer",
            "description": description.into(),
            "minimum": 0,
            "maximum": u64::MAX
        });
        self.properties.insert(name.clone(), props);
        self.required.push(name);
        self
    }

    /// Add a string argument
    pub fn arg_string(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let props = serde_json::json!({
            "type": "string",
            "description": description.into()
        });
        self.properties.insert(name.clone(), props);
        self.required.push(name);
        self
    }

    /// Add bytes argument (base64 encoded)
    pub fn arg_bytes(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let props = serde_json::json!({
            "type": "string",
            "description": description.into(),
            "contentEncoding": "base64"
        });
        self.properties.insert(name.clone(), props);
        self.required.push(name);
        self
    }

    /// Add optional argument
    pub fn arg_optional(
        mut self,
        name: impl Into<String>,
        prop_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let props = serde_json::json!({
            "type": prop_type.into(),
            "description": description.into()
        });
        self.properties.insert(name.into(), props);
        // Don't add to required
        self
    }

    pub fn build(self) -> McpTool {
        McpTool {
            name: self.name,
            description: self.description,
            input_schema: InputSchema {
                schema_type: "object".to_string(),
                properties: self.properties,
                required: self.required,
            },
        }
    }
}
