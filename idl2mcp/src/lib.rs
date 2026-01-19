//! IDL to MCP Schema Converter
//!
//! Converts Anchor IDL JSON to MCP schema format, enabling existing
//! Anchor programs to be discoverable by AI agents.

use anyhow::{Context, Result};
use mcpsol_core::{
    ArgType, McpSchema, McpSchemaBuilder, McpToolBuilder,
    generate_compact_schema,
};
use serde::Deserialize;
use std::collections::HashMap;

/// Anchor IDL root structure
#[derive(Debug, Deserialize)]
pub struct AnchorIdl {
    pub version: Option<String>,
    pub name: String,
    #[serde(default)]
    pub instructions: Vec<IdlInstruction>,
    #[serde(default)]
    pub accounts: Vec<IdlAccountDef>,
    #[serde(default)]
    pub types: Vec<IdlTypeDef>,
    #[serde(default)]
    pub events: Vec<IdlEvent>,
    #[serde(default)]
    pub errors: Vec<IdlError>,
    pub metadata: Option<IdlMetadata>,
}

/// IDL instruction definition
#[derive(Debug, Deserialize)]
pub struct IdlInstruction {
    pub name: String,
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default)]
    pub accounts: Vec<IdlAccountItem>,
    #[serde(default)]
    pub args: Vec<IdlArg>,
    #[serde(default)]
    pub returns: Option<IdlType>,
}

/// Account in an instruction (can be single or nested)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum IdlAccountItem {
    Single(IdlAccount),
    Composite(IdlAccounts),
}

/// Single account reference
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdlAccount {
    pub name: String,
    #[serde(default)]
    pub is_mut: bool,
    #[serde(default)]
    pub is_signer: bool,
    #[serde(default)]
    pub is_optional: bool,
    #[serde(default)]
    pub docs: Vec<String>,
    pub pda: Option<IdlPda>,
}

/// Composite accounts (nested struct)
#[derive(Debug, Deserialize)]
pub struct IdlAccounts {
    pub name: String,
    pub accounts: Vec<IdlAccountItem>,
}

/// PDA definition
#[derive(Debug, Deserialize)]
pub struct IdlPda {
    pub seeds: Vec<IdlSeed>,
    pub program: Option<IdlSeed>,
}

/// PDA seed
#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum IdlSeed {
    #[serde(rename = "const")]
    Const { value: serde_json::Value },
    #[serde(rename = "arg")]
    Arg { path: String },
    #[serde(rename = "account")]
    Account { path: String },
}

/// Instruction argument
#[derive(Debug, Deserialize)]
pub struct IdlArg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
}

/// IDL type
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum IdlType {
    Primitive(String),
    Option { option: Box<IdlType> },
    Vec { vec: Box<IdlType> },
    Array { array: (Box<IdlType>, usize) },
    Defined { defined: String },
    Generic { generic: String },
    Complex(HashMap<String, serde_json::Value>),
}

/// Account type definition
#[derive(Debug, Deserialize)]
pub struct IdlAccountDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlTypeDefTy,
    #[serde(default)]
    pub docs: Vec<String>,
}

/// Type definition
#[derive(Debug, Deserialize)]
pub struct IdlTypeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlTypeDefTy,
    #[serde(default)]
    pub docs: Vec<String>,
}

/// Type definition body
#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum IdlTypeDefTy {
    #[serde(rename = "struct")]
    Struct { fields: Vec<IdlField> },
    #[serde(rename = "enum")]
    Enum { variants: Vec<IdlEnumVariant> },
}

/// Struct field
#[derive(Debug, Deserialize)]
pub struct IdlField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    #[serde(default)]
    pub docs: Vec<String>,
}

/// Enum variant
#[derive(Debug, Deserialize)]
pub struct IdlEnumVariant {
    pub name: String,
    pub fields: Option<Vec<IdlField>>,
}

/// IDL event
#[derive(Debug, Deserialize)]
pub struct IdlEvent {
    pub name: String,
    pub fields: Vec<IdlField>,
}

/// IDL error
#[derive(Debug, Deserialize)]
pub struct IdlError {
    pub code: u32,
    pub name: String,
    pub msg: Option<String>,
}

/// IDL metadata
#[derive(Debug, Deserialize)]
pub struct IdlMetadata {
    pub address: Option<String>,
}

/// Convert IDL type to MCP ArgType
fn idl_type_to_arg_type(ty: &IdlType) -> ArgType {
    match ty {
        IdlType::Primitive(s) => match s.as_str() {
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
            "string" | "String" => ArgType::String,
            "pubkey" | "Pubkey" | "publicKey" => ArgType::Pubkey,
            "bytes" => ArgType::Bytes,
            _ => ArgType::String, // Default fallback
        },
        IdlType::Option { option } => idl_type_to_arg_type(option),
        IdlType::Vec { vec } => {
            // Vec<u8> is bytes, otherwise treat as string (JSON array)
            if matches!(vec.as_ref(), IdlType::Primitive(s) if s == "u8") {
                ArgType::Bytes
            } else {
                ArgType::String
            }
        }
        IdlType::Array { array: (inner, _) } => {
            if matches!(inner.as_ref(), IdlType::Primitive(s) if s == "u8") {
                ArgType::Bytes
            } else {
                ArgType::String
            }
        }
        IdlType::Defined { .. } => ArgType::String, // Custom types as JSON
        IdlType::Generic { .. } => ArgType::String,
        IdlType::Complex(_) => ArgType::String,
    }
}

/// Flatten nested account structures
fn flatten_accounts(items: &[IdlAccountItem], prefix: &str) -> Vec<(String, bool, bool)> {
    let mut result = Vec::new();

    for item in items {
        match item {
            IdlAccountItem::Single(acc) => {
                let name = if prefix.is_empty() {
                    acc.name.clone()
                } else {
                    format!("{}_{}", prefix, acc.name)
                };
                result.push((name, acc.is_signer, acc.is_mut));
            }
            IdlAccountItem::Composite(comp) => {
                let new_prefix = if prefix.is_empty() {
                    comp.name.clone()
                } else {
                    format!("{}_{}", prefix, comp.name)
                };
                result.extend(flatten_accounts(&comp.accounts, &new_prefix));
            }
        }
    }

    result
}

/// Convert Anchor IDL to MCP Schema
pub fn idl_to_mcp(idl: &AnchorIdl) -> McpSchema {
    let mut builder = McpSchemaBuilder::new(&idl.name);

    // Always add list_tools first
    builder = builder.add_tool(
        McpToolBuilder::new("list_tools")
            .description("List available MCP tools for this program")
            .build()
    );

    // Convert each instruction to an MCP tool
    for ix in &idl.instructions {
        let mut tool_builder = McpToolBuilder::new(&ix.name);

        // Use docs as description
        if !ix.docs.is_empty() {
            let desc = ix.docs.join(" ");
            tool_builder = tool_builder.description(desc);
        }

        // Add accounts
        let accounts = flatten_accounts(&ix.accounts, "");
        for (name, is_signer, is_writable) in accounts {
            tool_builder = tool_builder.account(&name, is_signer, is_writable);
        }

        // Add args
        for arg in &ix.args {
            let arg_type = idl_type_to_arg_type(&arg.ty);
            tool_builder = tool_builder.arg(&arg.name, arg_type);
        }

        builder = builder.add_tool(tool_builder.build());
    }

    builder.build()
}

/// Parse IDL JSON and convert to MCP schema
pub fn parse_idl_to_mcp(json: &str) -> Result<McpSchema> {
    let idl: AnchorIdl = serde_json::from_str(json)
        .context("Failed to parse IDL JSON")?;
    Ok(idl_to_mcp(&idl))
}

/// Parse IDL JSON and generate compact MCP schema JSON
pub fn convert_idl_to_mcp_json(idl_json: &str) -> Result<String> {
    let schema = parse_idl_to_mcp(idl_json)?;
    Ok(generate_compact_schema(&schema))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_IDL: &str = r#"{
        "version": "0.1.0",
        "name": "counter",
        "instructions": [
            {
                "name": "initialize",
                "docs": ["Initialize a new counter account"],
                "accounts": [
                    {"name": "counter", "isMut": true, "isSigner": true},
                    {"name": "authority", "isMut": false, "isSigner": true},
                    {"name": "systemProgram", "isMut": false, "isSigner": false}
                ],
                "args": []
            },
            {
                "name": "increment",
                "docs": ["Increment the counter by amount"],
                "accounts": [
                    {"name": "counter", "isMut": true, "isSigner": false},
                    {"name": "authority", "isMut": false, "isSigner": true}
                ],
                "args": [
                    {"name": "amount", "type": "u64"}
                ]
            }
        ],
        "accounts": [
            {
                "name": "Counter",
                "type": {
                    "kind": "struct",
                    "fields": [
                        {"name": "authority", "type": "pubkey"},
                        {"name": "count", "type": "u64"}
                    ]
                }
            }
        ]
    }"#;

    #[test]
    fn test_parse_idl() {
        let idl: AnchorIdl = serde_json::from_str(SAMPLE_IDL).unwrap();
        assert_eq!(idl.name, "counter");
        assert_eq!(idl.instructions.len(), 2);
    }

    #[test]
    fn test_idl_to_mcp() {
        let schema = parse_idl_to_mcp(SAMPLE_IDL).unwrap();

        assert_eq!(schema.name, "counter");
        // 3 tools: list_tools + initialize + increment
        assert_eq!(schema.tools.len(), 3);

        // Check list_tools is first
        assert_eq!(schema.tools[0].name, "list_tools");

        // Check initialize
        assert_eq!(schema.tools[1].name, "initialize");
        assert_eq!(schema.tools[1].accounts.len(), 3);
        assert_eq!(schema.tools[1].args.len(), 0);

        // Check increment
        assert_eq!(schema.tools[2].name, "increment");
        assert_eq!(schema.tools[2].accounts.len(), 2);
        assert_eq!(schema.tools[2].args.len(), 1);
        assert_eq!(schema.tools[2].args[0].name, "amount");
    }

    #[test]
    fn test_convert_to_json() {
        let json = convert_idl_to_mcp_json(SAMPLE_IDL).unwrap();

        println!("MCP JSON:\n{}", json);

        assert!(json.contains("\"name\":\"counter\""));
        assert!(json.contains("\"n\":\"list_tools\""));
        assert!(json.contains("\"n\":\"initialize\""));
        assert!(json.contains("\"n\":\"increment\""));
        assert!(json.contains("\"i\":\"Initialize a new counter account\""));
        assert!(json.contains("\"amount\":\"u64\""));
    }

    #[test]
    fn test_type_conversion() {
        let idl_json = r#"{
            "name": "types_test",
            "instructions": [{
                "name": "test_types",
                "accounts": [],
                "args": [
                    {"name": "amount", "type": "u64"},
                    {"name": "flag", "type": "bool"},
                    {"name": "key", "type": "pubkey"},
                    {"name": "data", "type": {"vec": "u8"}},
                    {"name": "name", "type": "string"},
                    {"name": "big", "type": "u128"}
                ]
            }]
        }"#;

        let schema = parse_idl_to_mcp(idl_json).unwrap();
        let test_types = &schema.tools[1];

        assert_eq!(test_types.args.len(), 6);
        assert_eq!(test_types.args[0].arg_type, ArgType::U64);
        assert_eq!(test_types.args[1].arg_type, ArgType::Bool);
        assert_eq!(test_types.args[2].arg_type, ArgType::Pubkey);
        assert_eq!(test_types.args[3].arg_type, ArgType::Bytes); // Vec<u8> -> Bytes
        assert_eq!(test_types.args[4].arg_type, ArgType::String);
        assert_eq!(test_types.args[5].arg_type, ArgType::U128);
    }
}
