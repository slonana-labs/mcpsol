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

                    let discriminator = instruction_discriminator(&tool_name);

                    // Extract function arguments (skip ctx which is first arg)
                    let mut args = Vec::new();
                    let mut accounts_type = None;

                    for (idx, input) in func.sig.inputs.iter().enumerate() {
                        if let FnArg::Typed(pat_type) = input {
                            // First arg is ctx - extract accounts type from it
                            if idx == 0 {
                                accounts_type = extract_accounts_type(&pat_type.ty);
                                continue;
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

                            args.push(ArgInfo {
                                name: arg_name,
                                rust_type,
                                json_type,
                                description: String::new(),
                            });
                        }
                    }

                    instructions.push(InstructionInfo {
                        fn_name,
                        tool_name,
                        tool_desc,
                        discriminator,
                        args,
                        accounts,
                        accounts_type,
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

/// Generate the instruction dispatcher (process_instruction function)
pub fn generate_dispatcher(
    mod_name: &Ident,
    instructions: &[InstructionInfo],
) -> TokenStream {
    let mut match_arms = Vec::new();

    for ix in instructions {
        let disc = &ix.discriminator;
        let fn_name = &ix.fn_name;

        // Generate argument parsing code
        let (arg_parsing, arg_names) = generate_arg_parsing(&ix.args);

        // Build the context if we have an accounts type
        let ctx_building = if let Some(ref accounts_type) = ix.accounts_type {
            let accounts_ty = Ident::new(accounts_type, fn_name.span());
            quote! {
                let ctx = mcpsol::context::Context::new(
                    program_id,
                    <#accounts_ty as mcpsol::context::Accounts>::try_accounts(program_id, accounts)?,
                    &[]  // remaining_accounts
                );
            }
        } else {
            quote! {}
        };

        // Generate the function call
        let fn_call = if ix.accounts_type.is_some() {
            if arg_names.is_empty() {
                quote! { #mod_name::#fn_name(ctx)? }
            } else {
                quote! { #mod_name::#fn_name(ctx, #(#arg_names),*)? }
            }
        } else {
            // No context - just call with args (rare case)
            if arg_names.is_empty() {
                quote! { #mod_name::#fn_name()? }
            } else {
                quote! { #mod_name::#fn_name(#(#arg_names),*)? }
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
        /// Process incoming instructions
        pub fn __mcpsol_process_instruction(
            program_id: &pinocchio::pubkey::Pubkey,
            accounts: &[pinocchio::account_info::AccountInfo],
            instruction_data: &[u8],
        ) -> pinocchio::ProgramResult {
            if instruction_data.len() < 8 {
                return Err(mcpsol::pinocchio::program_error::ProgramError::InvalidInstructionData);
            }

            let discriminator: [u8; 8] = instruction_data[..8]
                .try_into()
                .map_err(|_| mcpsol::pinocchio::program_error::ProgramError::InvalidInstructionData)?;

            let data = &instruction_data[8..];

            match discriminator {
                // Built-in list_tools instruction
                [#(#list_tools_disc),*] => {
                    pinocchio::program::set_return_data(#mod_name::MCP_SCHEMA_JSON);
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

/// Generate the list_tools instruction that returns MCP schema
pub fn generate_list_tools(schema_json: &str) -> TokenStream {
    let list_tools_disc = instruction_discriminator("list_tools");

    quote! {
        /// MCP schema as JSON bytes (auto-generated)
        pub const MCP_SCHEMA_JSON: &[u8] = #schema_json.as_bytes();

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
