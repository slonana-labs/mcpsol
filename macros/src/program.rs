//! Program entrypoint and instruction dispatcher generation.
//!
//! This module handles the `#[mcp_program]` macro expansion to generate:
//! - The program entrypoint
//! - Instruction discriminator routing
//! - The `list_tools` instruction for MCP schema discovery

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, Pat, Type};

use crate::discriminator::instruction_discriminator;

/// Information about a function argument
#[derive(Clone)]
pub struct ArgInfo {
    pub name: String,
    pub rust_type: String,
    pub json_type: String,
    #[allow(dead_code)] // Reserved for future schema expansion
    pub description: String,
}

/// Information about an account required by an instruction
#[derive(Clone)]
pub struct AccountMeta {
    pub name: String,
    pub is_signer: bool,
    pub is_writable: bool,
    #[allow(dead_code)] // Reserved for future schema expansion
    pub description: String,
}

/// Information about a single instruction extracted from the module
pub struct InstructionInfo {
    pub fn_name: Ident,
    pub tool_name: String,
    pub tool_desc: String,
    pub discriminator: [u8; 8],
    pub args: Vec<ArgInfo>,
    pub accounts: Vec<AccountMeta>,
    pub accounts_type: Option<String>, // e.g., "Initialize" from Context<Initialize>
    /// Whether to build Context wrapper. Auto-detected from first param or set via `context = true/false`
    pub use_context: bool,
}

/// Extract instruction info from functions marked with #[mcp_instruction]
pub fn extract_instructions(items: &[syn::Item]) -> Vec<InstructionInfo> {
    let mut instructions = Vec::new();

    for item in items {
        if let syn::Item::Fn(func) = item {
            // Check if this function has #[mcp_instruction] attribute
            for attr in &func.attrs {
                if attr.path().is_ident("mcp_instruction") {
                    let fn_name = func.sig.ident.clone();

                    // Parse attribute to get name and description
                    // Convert the entire attribute meta to string for parsing
                    let attr_str = match &attr.meta {
                        syn::Meta::List(list) => {
                            let s = list.tokens.to_string();
                            // Debug: uncomment to see what the token string looks like
                            // panic!("attr_str for {}: {}", fn_name, s);
                            s
                        }
                        syn::Meta::NameValue(nv) => quote::quote!(#nv).to_string(),
                        syn::Meta::Path(_) => String::new(),
                    };

                    let tool_name = extract_attr_value(&attr_str, "name")
                        .unwrap_or_else(|| fn_name.to_string());
                    let tool_desc = extract_attr_value(&attr_str, "description")
                        .unwrap_or_default();
                    let accounts_str = extract_attr_value(&attr_str, "accounts")
                        .unwrap_or_default();
                    let accounts = parse_accounts_attr(&accounts_str);

                    // Parse explicit context = true/false attribute
                    let explicit_context = extract_attr_value(&attr_str, "context");

                    let discriminator = instruction_discriminator(&tool_name);

                    // Extract function arguments
                    let mut args = Vec::new();
                    let mut accounts_type = None;
                    let mut detected_context = false;

                    for (idx, input) in func.sig.inputs.iter().enumerate() {
                        if let FnArg::Typed(pat_type) = input {
                            // First arg - check if it's Context<T>
                            if idx == 0 {
                                accounts_type = extract_accounts_type(&pat_type.ty);
                                // If we found a Context type, this instruction uses context
                                if accounts_type.is_some() {
                                    detected_context = true;
                                    continue; // Skip ctx in args
                                }
                                // Not a Context - include as arg (for no-Context signatures)
                            }

                            // Get argument name
                            let arg_name = if let Pat::Ident(pat_ident) = &*pat_type.pat {
                                pat_ident.ident.to_string()
                            } else {
                                format!("arg{}", idx)
                            };

                            // Get argument type
                            let rust_type = type_to_string(&pat_type.ty);
                            let json_type = rust_type_to_json_schema(&rust_type);

                            // Skip program_id and accounts slice for no-Context handlers
                            // They have signatures like: fn(program_id: &Pubkey, accounts: &[AccountInfo], ...)
                            if idx == 0 && (rust_type.contains("Pubkey") || rust_type.contains("&Pubkey")) {
                                continue; // Skip program_id
                            }
                            if idx == 1 && rust_type.contains("AccountInfo") {
                                continue; // Skip accounts slice
                            }

                            args.push(ArgInfo {
                                name: arg_name,
                                rust_type,
                                json_type,
                                description: String::new(),
                            });
                        }
                    }

                    // Determine whether to use Context:
                    // 1. Explicit `context = true` forces Context
                    // 2. Explicit `context = false` forces no Context
                    // 3. Otherwise, auto-detect from first parameter
                    let use_context = match explicit_context.as_deref() {
                        Some("true") => true,
                        Some("false") => false,
                        _ => detected_context, // Auto-detect
                    };

                    instructions.push(InstructionInfo {
                        fn_name,
                        tool_name,
                        tool_desc,
                        discriminator,
                        args,
                        accounts,
                        accounts_type,
                        use_context,
                    });
                }
            }
        }
    }

    instructions
}

/// Extract the accounts type from Context<'info, AccountsType<'info>>
fn extract_accounts_type(ty: &Type) -> Option<String> {
    let ty_str = quote!(#ty).to_string();
    // Look for Context < ... , AccountsType < ... > >
    if let Some(start) = ty_str.find("Context") {
        let after_context = &ty_str[start..];
        // Find the accounts type name (after the comma, before the next <)
        if let Some(comma) = after_context.find(',') {
            let after_comma = after_context[comma + 1..].trim();
            // Extract just the type name
            let type_name: String = after_comma
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !type_name.is_empty() {
                return Some(type_name);
            }
        }
    }
    None
}

/// Convert a Type to a string representation
fn type_to_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(" ", "")
}

/// Map Rust types to JSON Schema type objects
fn rust_type_to_json_schema(rust_type: &str) -> String {
    match rust_type {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" =>
            r#"{"type":"integer","minimum":0}"#.to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" =>
            r#"{"type":"integer"}"#.to_string(),
        "bool" =>
            r#"{"type":"boolean"}"#.to_string(),
        "String" =>
            r#"{"type":"string"}"#.to_string(),
        t if t.starts_with("Pubkey") || t.contains("Pubkey") =>
            r#"{"type":"string","format":"solana-pubkey","description":"Base58-encoded 32-byte public key"}"#.to_string(),
        t if t.starts_with("[u8;") =>
            r#"{"type":"string","contentEncoding":"base64"}"#.to_string(),
        t if t.starts_with("Vec<u8>") =>
            r#"{"type":"string","contentEncoding":"base64"}"#.to_string(),
        _ =>
            r#"{"type":"string"}"#.to_string(),
    }
}

/// Get the byte size of a Rust type for compile-time offset calculation.
///
/// Returns `Some(size)` for known fixed-size types, `None` for variable-size types.
/// Used by the macro to generate `EXPECTED_LEN` constants and compile-time offsets.
fn get_type_size(rust_type: &str) -> Option<usize> {
    match rust_type {
        "u8" | "i8" | "bool" => Some(1),
        "u16" | "i16" => Some(2),
        "u32" | "i32" => Some(4),
        "u64" | "i64" => Some(8),
        "u128" | "i128" => Some(16),
        t if t.starts_with("Pubkey") || t.contains("Pubkey") => Some(32),
        // Parse [u8; N] patterns
        t if t.starts_with("[u8;") => {
            // Extract N from "[u8;N]"
            let inner = t.trim_start_matches("[u8;").trim_end_matches(']');
            inner.trim().parse().ok()
        }
        // Variable-size types return None
        "String" | "Vec<u8>" => None,
        t if t.starts_with("Vec<") => None,
        // Unknown types - could be fixed size but we don't know
        _ => None,
    }
}

/// Calculate the total expected instruction data length for compile-time validation.
///
/// Returns `Some(len)` if all arguments have known fixed sizes, `None` otherwise.
/// The returned length includes the 8-byte discriminator.
fn calculate_expected_len(args: &[ArgInfo]) -> Option<usize> {
    let mut total: usize = 8; // discriminator
    for arg in args {
        match get_type_size(&arg.rust_type) {
            Some(size) => total += size,
            None => return None, // Variable-size arg, can't compute at compile time
        }
    }
    Some(total)
}

/// Calculate compile-time offsets for each argument.
///
/// Returns `Some(offsets)` where offsets[i] is the byte offset for arg[i],
/// or `None` if any argument has variable size.
fn calculate_arg_offsets(args: &[ArgInfo]) -> Option<Vec<usize>> {
    let mut offsets = Vec::with_capacity(args.len());
    let mut offset: usize = 8; // Start after discriminator

    for arg in args {
        offsets.push(offset);
        match get_type_size(&arg.rust_type) {
            Some(size) => offset += size,
            None => return None,
        }
    }
    Some(offsets)
}

/// Generate the instruction dispatcher (process_instruction function)
///
/// This generates an optimized dispatcher with:
/// - Single upfront bounds check for discriminator (8 bytes minimum)
/// - Unsafe direct discriminator read (~5 CU vs ~50 CU)
/// - Per-instruction bounds check using compile-time EXPECTED_LEN
/// - Unsafe argument reads at compile-time offsets (~5 CU vs ~70 CU per arg)
pub fn generate_dispatcher(
    mod_name: &Ident,
    instructions: &[InstructionInfo],
) -> TokenStream {
    let mut match_arms = Vec::new();

    for ix in instructions {
        let disc = &ix.discriminator;
        let fn_name = &ix.fn_name;

        // Generate optimized argument parsing code
        let (arg_parsing, arg_names) = generate_arg_parsing_optimized(&ix.args);

        // Build the context only if use_context is true
        let ctx_building = if ix.use_context {
            if let Some(ref accounts_type) = ix.accounts_type {
                let accounts_ty = Ident::new(accounts_type, fn_name.span());
                quote! {
                    let ctx = mcpsol::context::Context::new(
                        program_id,
                        <#accounts_ty as mcpsol::context::Accounts>::try_accounts(program_id, accounts)?,
                        &[]  // remaining_accounts
                    );
                }
            } else {
                // use_context = true but no accounts type detected - still build minimal context
                quote! {}
            }
        } else {
            // No context wrapper - maximum performance path (~30 CU total)
            quote! {}
        };

        // Generate the function call
        let fn_call = if ix.use_context {
            // With Context - pass ctx as first arg
            if arg_names.is_empty() {
                quote! { #mod_name::#fn_name(ctx)? }
            } else {
                quote! { #mod_name::#fn_name(ctx, #(#arg_names),*)? }
            }
        } else {
            // Without Context - pass (program_id, accounts, args...)
            // Handler signature: fn(program_id: &Pubkey, accounts: &[AccountInfo], ...args)
            if arg_names.is_empty() {
                quote! { #mod_name::#fn_name(program_id, accounts)? }
            } else {
                quote! { #mod_name::#fn_name(program_id, accounts, #(#arg_names),*)? }
            }
        };

        let arm = quote! {
            [#(#disc),*] => {
                #arg_parsing
                #ctx_building
                #fn_call;
                Ok(())
            }
        };
        match_arms.push(arm);
    }

    // Add list_tools discriminator
    let list_tools_disc = instruction_discriminator("list_tools");

    quote! {
        /// Process incoming instructions (optimized: ~30 CU framework overhead)
        pub fn __mcpsol_process_instruction(
            program_id: &pinocchio::pubkey::Pubkey,
            accounts: &[pinocchio::account_info::AccountInfo],
            instruction_data: &[u8],
        ) -> pinocchio::ProgramResult {
            // Single bounds check for discriminator
            if instruction_data.len() < 8 {
                return Err(mcpsol::pinocchio::program_error::ProgramError::InvalidInstructionData);
            }

            // SAFETY: Length >= 8 verified above
            // Optimization: Direct pointer read (~5 CU) vs try_into().map_err() (~50 CU)
            let discriminator = unsafe {
                *(instruction_data.as_ptr() as *const [u8; 8])
            };

            match discriminator {
                // Built-in list_tools instruction
                [#(#list_tools_disc),*] => {
                    pinocchio::program::set_return_data(#mod_name::MCP_SCHEMA_BYTES);
                    Ok(())
                }
                // User-defined instructions
                #(#match_arms)*
                _ => Err(mcpsol::pinocchio::program_error::ProgramError::InvalidInstructionData),
            }
        }
    }
}

/// Generate code to parse instruction arguments from data bytes
fn generate_arg_parsing(args: &[ArgInfo]) -> (TokenStream, Vec<Ident>) {
    if args.is_empty() {
        return (quote! { let _ = data; }, vec![]);
    }

    let mut parsing_code = Vec::new();
    let mut arg_names = Vec::new();
    let offset_code = quote! { let mut __offset: usize = 0; };

    for arg in args {
        let arg_name = Ident::new(&arg.name, proc_macro2::Span::call_site());
        arg_names.push(arg_name.clone());

        let parse_expr = match arg.rust_type.as_str() {
            "u8" => quote! {
                let #arg_name: u8 = data.get(__offset)
                    .copied()
                    .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;
                __offset += 1;
            },
            "u16" => quote! {
                let #arg_name: u16 = u16::from_le_bytes(
                    data.get(__offset..__offset + 2)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 2;
            },
            "u32" => quote! {
                let #arg_name: u32 = u32::from_le_bytes(
                    data.get(__offset..__offset + 4)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 4;
            },
            "u64" => quote! {
                let #arg_name: u64 = u64::from_le_bytes(
                    data.get(__offset..__offset + 8)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 8;
            },
            "i8" => quote! {
                let #arg_name: i8 = data.get(__offset)
                    .map(|&b| b as i8)
                    .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;
                __offset += 1;
            },
            "i16" => quote! {
                let #arg_name: i16 = i16::from_le_bytes(
                    data.get(__offset..__offset + 2)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 2;
            },
            "i32" => quote! {
                let #arg_name: i32 = i32::from_le_bytes(
                    data.get(__offset..__offset + 4)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 4;
            },
            "i64" => quote! {
                let #arg_name: i64 = i64::from_le_bytes(
                    data.get(__offset..__offset + 8)
                        .and_then(|s| s.try_into().ok())
                        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?
                );
                __offset += 8;
            },
            "bool" => quote! {
                let #arg_name: bool = data.get(__offset)
                    .map(|&b| b != 0)
                    .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;
                __offset += 1;
            },
            // Default: try to read as raw bytes (for types we don't recognize)
            _ => quote! {
                // Unknown type - skip parsing, caller must handle
                let #arg_name = ();
            },
        };

        parsing_code.push(parse_expr);
    }

    let combined = quote! {
        #offset_code
        #(#parsing_code)*
    };

    (combined, arg_names)
}

/// Generate optimized code to parse instruction arguments from data bytes.
///
/// This version uses:
/// - Compile-time offset calculation (no mutable __offset)
/// - Single bounds check with EXPECTED_LEN const
/// - Unsafe direct reads with SAFETY comments
/// - debug_assert! for extra verification in debug builds
fn generate_arg_parsing_optimized(args: &[ArgInfo]) -> (TokenStream, Vec<Ident>) {
    if args.is_empty() {
        return (quote! {}, vec![]);
    }

    // Try to calculate compile-time offsets
    let offsets = match calculate_arg_offsets(args) {
        Some(offsets) => offsets,
        None => {
            // Fall back to legacy parsing for variable-size args
            return generate_arg_parsing(args);
        }
    };

    let expected_len = match calculate_expected_len(args) {
        Some(len) => len,
        None => {
            // Fall back to legacy parsing
            return generate_arg_parsing(args);
        }
    };

    let mut parsing_code = Vec::new();
    let mut arg_names = Vec::new();

    // Generate compile-time length check
    let bounds_check = quote! {
        // Compile-time constant for expected instruction data length
        const __EXPECTED_LEN: usize = #expected_len;
        if instruction_data.len() < __EXPECTED_LEN {
            return Err(mcpsol::pinocchio::program_error::ProgramError::InvalidInstructionData);
        }
    };
    parsing_code.push(bounds_check);

    // Generate optimized reads at compile-time offsets
    for (i, arg) in args.iter().enumerate() {
        let arg_name = Ident::new(&arg.name, proc_macro2::Span::call_site());
        arg_names.push(arg_name.clone());
        let offset = offsets[i];

        let parse_expr = match arg.rust_type.as_str() {
            "u8" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset < instruction_data.len());
                let #arg_name: u8 = unsafe {
                    *instruction_data.as_ptr().add(#offset)
                };
            },
            "u16" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 2 <= instruction_data.len());
                let #arg_name: u16 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const u16)
                };
            },
            "u32" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 4 <= instruction_data.len());
                let #arg_name: u32 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const u32)
                };
            },
            "u64" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 8 <= instruction_data.len());
                let #arg_name: u64 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const u64)
                };
            },
            "i8" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset < instruction_data.len());
                let #arg_name: i8 = unsafe {
                    *instruction_data.as_ptr().add(#offset) as i8
                };
            },
            "i16" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 2 <= instruction_data.len());
                let #arg_name: i16 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const i16)
                };
            },
            "i32" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 4 <= instruction_data.len());
                let #arg_name: i32 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const i32)
                };
            },
            "i64" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 8 <= instruction_data.len());
                let #arg_name: i64 = unsafe {
                    core::ptr::read_unaligned(instruction_data.as_ptr().add(#offset) as *const i64)
                };
            },
            "bool" => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset < instruction_data.len());
                let #arg_name: bool = unsafe {
                    *instruction_data.as_ptr().add(#offset) != 0
                };
            },
            t if t.starts_with("Pubkey") || t.contains("Pubkey") => quote! {
                // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
                debug_assert!(#offset + 32 <= instruction_data.len());
                let #arg_name = unsafe {
                    let bytes: [u8; 32] = core::ptr::read_unaligned(
                        instruction_data.as_ptr().add(#offset) as *const [u8; 32]
                    );
                    pinocchio::pubkey::Pubkey::from(bytes)
                };
            },
            // Unknown fixed-size type - use legacy parsing
            _ => {
                // Fall back to legacy for this unknown type
                return generate_arg_parsing(args);
            }
        };

        parsing_code.push(parse_expr);
    }

    let combined = quote! {
        #(#parsing_code)*
    };

    (combined, arg_names)
}

/// Generate the list_tools instruction that returns MCP schema
pub fn generate_list_tools(schema_json: &str) -> TokenStream {
    let list_tools_disc = instruction_discriminator("list_tools");

    // Convert schema JSON to byte array literal for zero-cost access
    let schema_bytes: Vec<u8> = schema_json.bytes().collect();

    quote! {
        /// MCP schema as JSON bytes (auto-generated, zero runtime overhead)
        pub const MCP_SCHEMA_BYTES: &[u8] = &[#(#schema_bytes),*];

        /// Legacy alias for backwards compatibility
        pub const MCP_SCHEMA_JSON: &[u8] = MCP_SCHEMA_BYTES;

        /// Discriminator for list_tools instruction
        pub const LIST_TOOLS_DISCRIMINATOR: [u8; 8] = [#(#list_tools_disc),*];
    }
}

/// Generate the entrypoint macro invocation
pub fn generate_entrypoint() -> TokenStream {
    quote! {
        pinocchio::entrypoint!(__mcpsol_process_instruction);
    }
}

fn extract_attr_value(attr_str: &str, key: &str) -> Option<String> {
    // Handle various whitespace patterns around = sign
    // The tokenizer might produce "key = \"", "key =\n\"", etc.
    for pattern in [
        format!("{} = \"", key),
        format!("{} =\n\"", key),
        format!("{}= \"", key),
        format!("{}=\n\"", key),
        format!("{} =\"", key),
        format!("{}=\"", key),
    ] {
        if let Some(start) = attr_str.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = attr_str[value_start..].find('"') {
                return Some(attr_str[value_start..value_start + end].to_string());
            }
        }
    }
    None
}

/// Parse accounts attribute string into AccountMeta list
/// Format: "name:flags, name:flags" where flags can be "signer", "mut", or "signer,mut"
fn parse_accounts_attr(accounts_str: &str) -> Vec<AccountMeta> {
    if accounts_str.is_empty() {
        return Vec::new();
    }

    accounts_str
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let (name, flags) = if let Some(colon_pos) = part.find(':') {
                (part[..colon_pos].trim(), part[colon_pos + 1..].trim())
            } else {
                (part, "")
            };

            let is_signer = flags.contains("signer");
            let is_writable = flags.contains("mut");

            Some(AccountMeta {
                name: name.to_string(),
                is_signer,
                is_writable,
                description: String::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_attr_value() {
        // Test different spacing patterns
        let attr1 = r#"name = "increment" , description = "Increase counter""#;
        let attr2 = r#"name= "increment", description= "Increase counter""#;
        let attr3 = r#"name ="increment" , description ="Increase counter""#;
        let attr4 = r#"name="increment",description="Increase counter""#;
        
        for (i, attr) in [attr1, attr2, attr3, attr4].iter().enumerate() {
            println!("Test {}: {:?}", i+1, attr);
            let name = extract_attr_value(attr, "name");
            let desc = extract_attr_value(attr, "description");
            println!("  name: {:?}, description: {:?}", name, desc);
            assert!(name.is_some(), "name should be found in test {}", i+1);
            assert!(desc.is_some(), "description should be found in test {}", i+1);
        }
    }
}
