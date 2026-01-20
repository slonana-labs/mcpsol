//! MCP schema generation from macro metadata.
//!
//! Generates JSON schema at compile time from instruction and account definitions.

use crate::program::InstructionInfo;

/// Generate MCP schema JSON string from extracted metadata
/// Note: Solana return_data limit is 1024 bytes, so we keep schema compact
pub fn generate_schema_json(
    program_name: &str,
    _program_desc: &str,  // Omitted to save space
    instructions: &[InstructionInfo],
) -> String {
    let mut tools = Vec::new();

    for ix in instructions {
        let tool = generate_tool_schema(ix);
        tools.push(tool);
    }

    // Add list_tools as a built-in tool (compact format matching other tools)
    tools.push(r#"{"n":"list_tools","d":"42195e6a55fd41c0"}"#.to_string());

    // Compact format - omit description and resources to stay under 1024 bytes
    format!(
        r#"{{"v":"2024-11-05","name":"{}","tools":[{}]}}"#,
        escape_json(program_name),
        tools.join(","),
    )
}

/// Generate a single tool's schema (compact format for 1024 byte limit)
fn generate_tool_schema(ix: &InstructionInfo) -> String {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    // Add accounts as pubkey properties (compact: just type, no description)
    for acc in &ix.accounts {
        // Compact: mark signer/writable in property name suffix
        let suffix = match (acc.is_signer, acc.is_writable) {
            (true, true) => "_sw",
            (true, false) => "_s",
            (false, true) => "_w",
            (false, false) => "",
        };
        let escaped_acc_name = escape_json(&acc.name);
        let prop = format!(
            r#""{}{}":"pubkey""#,
            escaped_acc_name, suffix
        );
        properties.push(prop);
        required.push(format!(r#""{}{}""#, escaped_acc_name, suffix));
    }

    // Add instruction arguments (compact types)
    for arg in &ix.args {
        // Map to compact type names
        let compact_type = if arg.json_type.contains("integer") {
            "int"
        } else if arg.json_type.contains("boolean") {
            "bool"
        } else {
            "str"
        };
        let escaped_arg_name = escape_json(&arg.name);
        let prop = format!(r#""{}":"{}""#, escaped_arg_name, compact_type);
        properties.push(prop);
        required.push(format!(r#""{}""#, escaped_arg_name));
    }

    // Discriminator as hex (essential for calling)
    let disc_hex: String = ix.discriminator.iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    // Add description if present (compact: "i" = info)
    let desc_part = if !ix.tool_desc.is_empty() {
        format!(r#","i":"{}""#, escape_json(&ix.tool_desc))
    } else {
        String::new()
    };

    // Compact format: n=name, i=info (optional), d=discriminator, p=props, r=required
    let escaped_name = escape_json(&ix.tool_name);
    if properties.is_empty() {
        format!(
            r#"{{"n":"{}"{},"d":"{}"}}"#,
            escaped_name,
            desc_part,
            disc_hex,
        )
    } else {
        format!(
            r#"{{"n":"{}"{},"d":"{}","p":{{{}}},"r":[{}]}}"#,
            escaped_name,
            desc_part,
            disc_hex,
            properties.join(","),
            required.join(","),
        )
    }
}

/// Escape special characters for JSON string
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::{AccountMeta, ArgInfo, InstructionInfo};
    use crate::discriminator::instruction_discriminator;
    use syn::Ident;
    use proc_macro2::Span;

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_generate_compact_schema() {
        let instructions = vec![
            InstructionInfo {
                fn_name: Ident::new("increment", Span::call_site()),
                tool_name: "increment".to_string(),
                tool_desc: "Increase counter value".to_string(),
                discriminator: instruction_discriminator("increment"),
                args: vec![
                    ArgInfo {
                        name: "amount".to_string(),
                        rust_type: "u64".to_string(),
                        json_type: r#"{"type":"integer","minimum":0}"#.to_string(),
                        description: String::new(),
                    },
                ],
                accounts: vec![
                    AccountMeta {
                        name: "counter".to_string(),
                        is_signer: false,
                        is_writable: true,
                        description: String::new(),
                    },
                    AccountMeta {
                        name: "authority".to_string(),
                        is_signer: true,
                        is_writable: false,
                        description: String::new(),
                    },
                ],
                accounts_type: Some("Modify".to_string()),
                use_context: true,
            },
        ];

        let schema = generate_schema_json("test_program", "A test program", &instructions);

        // Verify compact format
        assert!(schema.contains(r#""v":"2024-11-05""#));
        assert!(schema.contains(r#""name":"test_program""#));
        assert!(schema.contains(r#""n":"increment""#));  // tool name

        // Verify accounts with suffix markers (_w = writable, _s = signer)
        assert!(schema.contains(r#""counter_w":"pubkey""#));
        assert!(schema.contains(r#""authority_s":"pubkey""#));

        // Verify args with compact types
        assert!(schema.contains(r#""amount":"int""#));

        // Verify discriminator
        assert!(schema.contains(r#""d":"0b12680968ae3b21""#));

        // Check schema size is under 1024 bytes
        assert!(schema.len() < 1024, "Schema too large: {} bytes", schema.len());

        // Print for manual inspection
        println!("Generated schema ({} bytes):\n{}", schema.len(), schema);
    }
}
