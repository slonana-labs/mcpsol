//! JSON generation for MCP schemas
//!
//! Provides both compact and verbose JSON formats:
//! - Compact: Abbreviated keys, fits in 1KB return_data
//! - Verbose: Full descriptions with pagination support

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec, format};

use crate::{McpSchema, McpTool, PROTOCOL_VERSION};
use crate::discriminator::discriminator_to_hex;

// ============================================================================
// Paginated Verbose Schema (for AI agents with full descriptions)
// ============================================================================

/// Generate a paginated schema response with full descriptions
///
/// Returns one tool per page with full parameter descriptions.
/// Use cursor to iterate through tools.
///
/// # Arguments
/// * `schema` - The full schema
/// * `cursor` - Page number (0-indexed), corresponds to tool index
///
/// # Returns
/// JSON string with single tool and optional nextCursor
pub fn generate_paginated_schema(schema: &McpSchema, cursor: u8) -> String {
    let cursor_idx = cursor as usize;

    let mut json = String::with_capacity(900);
    json.push_str("{\"v\":\"");
    json.push_str(PROTOCOL_VERSION);
    json.push_str("\",\"name\":\"");
    escape_json_into(&schema.name, &mut json);
    json.push_str("\",\"tools\":[");

    // Get the tool at cursor index
    if let Some(tool) = schema.tools.get(cursor_idx) {
        generate_verbose_tool(tool, &mut json);
    }

    json.push(']');

    // Add nextCursor if more tools exist
    if cursor_idx + 1 < schema.tools.len() {
        json.push_str(",\"nextCursor\":\"");
        // Write cursor as string number
        let next = cursor_idx + 1;
        let mut tmp = [0u8; 3];
        let s = format_cursor(next, &mut tmp);
        json.push_str(s);
        json.push('"');
    }

    json.push('}');
    json
}

/// Format cursor number to string (no_std compatible)
fn format_cursor(n: usize, buf: &mut [u8; 3]) -> &str {
    if n == 0 {
        return "0";
    }
    let mut i = 3;
    let mut num = n;
    while num > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("0")
}

/// Generate verbose JSON for a single tool with full descriptions
fn generate_verbose_tool(tool: &McpTool, json: &mut String) {
    json.push_str("{\"name\":\"");
    escape_json_into(&tool.name, json);
    json.push('"');

    // Description
    if let Some(ref desc) = tool.description {
        json.push_str(",\"description\":\"");
        escape_json_into(desc, json);
        json.push('"');
    }

    // Discriminator (for Solana instruction routing)
    json.push_str(",\"discriminator\":\"");
    let hex = discriminator_to_hex(&tool.discriminator);
    json.push_str(core::str::from_utf8(&hex).unwrap_or("0000000000000000"));
    json.push('"');

    // Parameters object with full descriptions
    if !tool.accounts.is_empty() || !tool.args.is_empty() {
        json.push_str(",\"parameters\":{");

        let mut first = true;

        // Accounts
        for acc in &tool.accounts {
            if !first {
                json.push(',');
            }
            first = false;

            json.push('"');
            escape_json_into(&acc.name, json);
            json.push_str("\":{\"type\":\"pubkey\"");

            if acc.is_signer {
                json.push_str(",\"signer\":true");
            }
            if acc.is_writable {
                json.push_str(",\"writable\":true");
            }
            if let Some(ref desc) = acc.description {
                json.push_str(",\"description\":\"");
                escape_json_into(desc, json);
                json.push('"');
            }
            json.push('}');
        }

        // Args
        for arg in &tool.args {
            if !first {
                json.push(',');
            }
            first = false;

            json.push('"');
            escape_json_into(&arg.name, json);
            json.push_str("\":{\"type\":\"");
            json.push_str(arg.arg_type.compact_name());
            json.push('"');

            if let Some(ref desc) = arg.description {
                json.push_str(",\"description\":\"");
                escape_json_into(desc, json);
                json.push('"');
            }
            json.push('}');
        }

        json.push('}');
    }

    json.push('}');
}

/// Generate paginated schema as bytes for set_return_data
pub fn generate_paginated_schema_bytes(schema: &McpSchema, cursor: u8) -> Vec<u8> {
    generate_paginated_schema(schema, cursor).into_bytes()
}

// ============================================================================
// Compact Schema (backwards compatible, all tools in one response)
// ============================================================================

/// Generate compact MCP schema JSON
///
/// Format:
/// ```json
/// {"v":"2024-11-05","name":"program","tools":[...]}
/// ```
pub fn generate_compact_schema(schema: &McpSchema) -> String {
    let mut json = String::with_capacity(800);
    json.push_str("{\"v\":\"");
    json.push_str(PROTOCOL_VERSION);
    json.push_str("\",\"name\":\"");
    escape_json_into(&schema.name, &mut json);
    json.push_str("\",\"tools\":[");

    for (i, tool) in schema.tools.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        generate_tool_json(tool, &mut json);
    }

    json.push_str("]}");
    json
}

/// Generate JSON for a single tool
fn generate_tool_json(tool: &McpTool, json: &mut String) {
    json.push_str("{\"n\":\"");
    escape_json_into(&tool.name, json);
    json.push('"');

    // Include description if present (key: "i" for info)
    if let Some(ref desc) = tool.description {
        json.push_str(",\"i\":\"");
        escape_json_into(desc, json);
        json.push('"');
    }

    json.push_str(",\"d\":\"");

    // Discriminator as hex
    let hex = discriminator_to_hex(&tool.discriminator);
    json.push_str(core::str::from_utf8(&hex).unwrap_or("0000000000000000"));

    // Only include p and r if there are properties
    if tool.accounts.is_empty() && tool.args.is_empty() {
        json.push_str("\"}");
        return;
    }

    json.push_str("\",\"p\":{");

    // Collect all properties (accounts + args)
    let mut first = true;
    let mut required = Vec::new();

    // Add accounts with suffixes
    for acc in &tool.accounts {
        if !first {
            json.push(',');
        }
        first = false;

        json.push('"');
        escape_json_into(&acc.name, json);
        json.push_str(acc.suffix());
        json.push_str("\":\"pubkey\"");

        // Build key for required array
        let mut key = String::new();
        escape_json_into(&acc.name, &mut key);
        key.push_str(acc.suffix());
        required.push(key);
    }

    // Add args
    for arg in &tool.args {
        if !first {
            json.push(',');
        }
        first = false;

        json.push('"');
        escape_json_into(&arg.name, json);
        json.push_str("\":\"");
        json.push_str(arg.arg_type.compact_name());
        json.push('"');

        let mut key = String::new();
        escape_json_into(&arg.name, &mut key);
        required.push(key);
    }

    json.push_str("},\"r\":[");

    // Required array
    for (i, r) in required.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push('"');
        json.push_str(r);
        json.push('"');
    }

    json.push_str("]}");
}

/// Escape JSON special characters into a string buffer
fn escape_json_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
}

/// Generate schema as bytes for set_return_data
pub fn generate_schema_bytes(schema: &McpSchema) -> Vec<u8> {
    generate_compact_schema(schema).into_bytes()
}

/// Estimate schema size without generating full JSON
pub fn estimate_schema_size(schema: &McpSchema) -> usize {
    let mut size = 50; // Base overhead
    size += schema.name.len();

    for tool in &schema.tools {
        size += estimate_single_tool_size(Some(tool));
    }

    size
}

/// Estimate the size of a single tool's JSON representation.
///
/// Used for pre-allocating buffers in paginated schema generation.
/// Returns 0 if tool is None.
pub fn estimate_single_tool_size(tool: Option<&McpTool>) -> usize {
    let tool = match tool {
        Some(t) => t,
        None => return 0,
    };

    let mut size = 30; // Tool overhead: {"n":"...","d":"..."}
    size += tool.name.len();
    size += 16; // Discriminator hex (8 bytes = 16 hex chars)

    if let Some(ref desc) = tool.description {
        size += desc.len() + 6; // ,"i":"..." overhead
    }

    // Accounts: "name_suffix":"pubkey"
    for acc in &tool.accounts {
        size += acc.name.len() + 15; // name + suffix + "pubkey" + quotes + colon
    }

    // Args: "name":"type"
    for arg in &tool.args {
        size += arg.name.len() + 10; // name + type + quotes + colon
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpSchemaBuilder, McpToolBuilder, ArgType};

    #[test]
    fn test_compact_schema_generation() {
        let schema = McpSchemaBuilder::new("test_program")
            .add_tool(
                McpToolBuilder::new("transfer")
                    .description("Transfer tokens between accounts")
                    .signer_writable("from")
                    .writable("to")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_compact_schema(&schema);

        assert!(json.contains("\"v\":\"2024-11-05\""));
        assert!(json.contains("\"name\":\"test_program\""));
        assert!(json.contains("\"n\":\"transfer\""));
        assert!(json.contains("\"i\":\"Transfer tokens between accounts\""));
        assert!(json.contains("\"from_sw\":\"pubkey\""));
        assert!(json.contains("\"to_w\":\"pubkey\""));
        assert!(json.contains("\"amount\":\"u64\""));
    }

    #[test]
    fn test_schema_fits_return_data_with_descriptions() {
        // Simulate a realistic program with 4 tools including descriptions
        let schema = McpSchemaBuilder::new("counter")
            .add_tool(
                McpToolBuilder::new("list_tools")
                    .description("List available MCP tools")
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("initialize")
                    .description("Create a new counter")
                    .signer_writable("counter")
                    .signer("authority")
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("increment")
                    .description("Add to counter value")
                    .writable("counter")
                    .signer("authority")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("decrement")
                    .description("Subtract from counter")
                    .writable("counter")
                    .signer("authority")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_compact_schema(&schema);
        let size = json.len();

        println!("Schema JSON ({} bytes):\n{}", size, json);
        assert!(size <= 1024, "Schema size {} exceeds 1024 byte limit", size);
    }

    #[test]
    fn test_description_escaping() {
        let schema = McpSchemaBuilder::new("test")
            .add_tool(
                McpToolBuilder::new("test")
                    .description("A \"quoted\" description with\\backslash")
                    .build()
            )
            .build();

        let json = generate_compact_schema(&schema);
        assert!(json.contains(r#"\"quoted\""#));
        assert!(json.contains(r#"\\"#));
    }

    #[test]
    fn test_tool_without_params() {
        // list_tools has no accounts or args - should not include empty p:{} and r:[]
        let schema = McpSchemaBuilder::new("test")
            .add_tool(
                McpToolBuilder::new("list_tools")
                    .description("List tools")
                    .build()
            )
            .build();

        let json = generate_compact_schema(&schema);
        println!("JSON: {}", json);

        // Should end with just the discriminator, no p or r
        assert!(json.contains(r#""n":"list_tools""#));
        assert!(json.contains(r#""i":"List tools""#));
        assert!(!json.contains(r#""p":{}"#), "Should not have empty params");
        assert!(!json.contains(r#""r":[]"#), "Should not have empty required");
    }

    #[test]
    fn test_name_escaping() {
        // Test that names with special chars are escaped
        let schema = McpSchemaBuilder::new("test\"program")
            .add_tool(
                McpToolBuilder::new("test\"tool")
                    .writable("acc\"name")
                    .arg("arg\"name", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_compact_schema(&schema);
        println!("JSON: {}", json);

        assert!(json.contains(r#""name":"test\"program""#));
        assert!(json.contains(r#""n":"test\"tool""#));
        assert!(json.contains(r#""acc\"name_w":"pubkey""#));
        assert!(json.contains(r#""arg\"name":"u64""#));
    }

    // ========================================================================
    // Paginated Schema Tests
    // ========================================================================

    #[test]
    fn test_paginated_schema_first_page() {
        let schema = McpSchemaBuilder::new("counter")
            .add_tool(
                McpToolBuilder::new("initialize")
                    .description("Create a new counter account")
                    .signer_writable_desc("counter", "The counter PDA to create")
                    .signer_desc("authority", "Who can modify this counter")
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("increment")
                    .description("Add amount to counter")
                    .writable_desc("counter", "Counter to modify")
                    .signer_desc("authority", "Must match counter authority")
                    .arg_desc("amount", "Value to add", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_paginated_schema(&schema, 0);
        println!("Page 0 ({} bytes):\n{}", json.len(), json);

        // First page should have first tool
        assert!(json.contains("\"name\":\"initialize\""));
        assert!(json.contains("\"description\":\"Create a new counter account\""));
        assert!(json.contains("\"counter\":{\"type\":\"pubkey\",\"signer\":true,\"writable\":true"));
        assert!(json.contains("\"description\":\"The counter PDA to create\""));

        // Should have nextCursor
        assert!(json.contains("\"nextCursor\":\"1\""));

        // Should fit in 1024 bytes
        assert!(json.len() <= 1024, "Page size {} exceeds limit", json.len());
    }

    #[test]
    fn test_paginated_schema_last_page() {
        let schema = McpSchemaBuilder::new("counter")
            .add_tool(
                McpToolBuilder::new("initialize")
                    .description("Create counter")
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("increment")
                    .description("Add to counter")
                    .arg_desc("amount", "Value to add", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_paginated_schema(&schema, 1);
        println!("Page 1 ({} bytes):\n{}", json.len(), json);

        // Last page should have second tool
        assert!(json.contains("\"name\":\"increment\""));
        assert!(json.contains("\"description\":\"Add to counter\""));

        // Should NOT have nextCursor (last page)
        assert!(!json.contains("nextCursor"));
    }

    #[test]
    fn test_paginated_schema_out_of_bounds() {
        let schema = McpSchemaBuilder::new("counter")
            .add_tool(
                McpToolBuilder::new("initialize")
                    .description("Create counter")
                    .build()
            )
            .build();

        // Cursor beyond available tools
        let json = generate_paginated_schema(&schema, 5);
        println!("Page 5 (out of bounds):\n{}", json);

        // Should return empty tools array, no nextCursor
        assert!(json.contains("\"tools\":[]"));
        assert!(!json.contains("nextCursor"));
    }

    #[test]
    fn test_paginated_verbose_fits_1kb() {
        // Even with full descriptions, single tool should fit
        let schema = McpSchemaBuilder::new("my_defi_protocol")
            .add_tool(
                McpToolBuilder::new("swap_tokens")
                    .description("Swap tokens using the AMM. Calculates optimal route automatically.")
                    .signer_writable_desc("user_token_a", "User's source token account to swap from")
                    .writable_desc("user_token_b", "User's destination token account to receive")
                    .writable_desc("pool_token_a", "Pool's token A reserve account")
                    .writable_desc("pool_token_b", "Pool's token B reserve account")
                    .signer_desc("user", "The user performing the swap")
                    .account("token_program", false, false)
                    .arg_desc("amount_in", "Amount of token A to swap", ArgType::U64)
                    .arg_desc("min_amount_out", "Minimum token B to receive (slippage)", ArgType::U64)
                    .build()
            )
            .build();

        let json = generate_paginated_schema(&schema, 0);
        println!("Verbose tool ({} bytes):\n{}", json.len(), json);

        assert!(json.len() <= 1024, "Verbose single tool {} exceeds 1024", json.len());
    }

    #[test]
    fn test_format_cursor() {
        let mut buf = [0u8; 3];
        assert_eq!(format_cursor(0, &mut buf), "0");
        assert_eq!(format_cursor(1, &mut buf), "1");
        assert_eq!(format_cursor(9, &mut buf), "9");
        assert_eq!(format_cursor(10, &mut buf), "10");
        assert_eq!(format_cursor(99, &mut buf), "99");
        assert_eq!(format_cursor(255, &mut buf), "255");
    }

    // ========================================================================
    // CU Optimization Tests - Verify JSON output remains identical
    // ========================================================================

    #[test]
    fn test_cached_pages_identical_output() {
        // Build a typical schema
        let schema = McpSchemaBuilder::new("counter")
            .add_tool(
                McpToolBuilder::new("initialize")
                    .description("Create counter")
                    .signer_writable("counter")
                    .signer("authority")
                    .build()
            )
            .add_tool(
                McpToolBuilder::new("increment")
                    .description("Add to counter")
                    .writable("counter")
                    .signer("authority")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .build();

        // Generate pages directly
        let direct_page_0 = generate_paginated_schema_bytes(&schema, 0);
        let direct_page_1 = generate_paginated_schema_bytes(&schema, 1);

        // Generate pages via CachedSchemaPages
        let cached = crate::CachedSchemaPages::from_schema(&schema);
        let cached_page_0 = cached.get_page(0);
        let cached_page_1 = cached.get_page(1);

        // Verify byte-for-byte identical output
        assert_eq!(
            direct_page_0, cached_page_0,
            "Page 0 output differs between direct and cached generation"
        );
        assert_eq!(
            direct_page_1, cached_page_1,
            "Page 1 output differs between direct and cached generation"
        );

        // Verify content is valid JSON
        let json_0 = String::from_utf8_lossy(cached_page_0);
        let json_1 = String::from_utf8_lossy(cached_page_1);
        assert!(json_0.starts_with("{\"v\":"), "Page 0 should be valid JSON");
        assert!(json_1.starts_with("{\"v\":"), "Page 1 should be valid JSON");
    }

    #[test]
    fn test_presized_buffer_identical_output() {
        // Verify that pre-sized buffer optimization produces identical output
        let schema = McpSchemaBuilder::new("test_program")
            .add_tool(
                McpToolBuilder::new("action")
                    .description("Do something")
                    .signer_writable("account")
                    .arg("value", ArgType::U64)
                    .build()
            )
            .build();

        // Generate multiple times - should be identical
        let output_1 = generate_compact_schema(&schema);
        let output_2 = generate_compact_schema(&schema);
        let output_3 = generate_compact_schema(&schema);

        assert_eq!(output_1, output_2, "Repeated generation should be identical");
        assert_eq!(output_2, output_3, "Repeated generation should be identical");

        // Verify JSON structure
        assert!(output_1.contains("\"v\":\"2024-11-05\""));
        assert!(output_1.contains("\"name\":\"test_program\""));
        assert!(output_1.contains("\"n\":\"action\""));
    }
}
