//! Procedural macros for the mcpsol SDK.
//!
//! Provides attribute and derive macros for building MCP-native Solana programs.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, ItemFn, ItemMod, Type};

mod discriminator;
mod mcp_gen;
mod program;

use discriminator::{account_discriminator, instruction_discriminator};

/// Marks a module as an MCP-enabled Solana program.
///
/// Generates:
/// - Program entrypoint
/// - MCP schema generation
/// - Instruction dispatcher
///
/// # Example
///
/// ```rust,ignore
/// #[mcp_program(
///     name = "my_program",
///     description = "A sample MCP Solana program"
/// )]
/// pub mod my_program {
///     use super::*;
///
///     #[mcp_instruction]
///     pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_program(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemMod);
    let attrs = parse_program_attrs(attr);

    let mod_name = &input.ident;
    let mod_vis = &input.vis;
    let mod_content = &input.content;

    let program_name = attrs.name.unwrap_or_else(|| mod_name.to_string());
    let program_desc = attrs.description.unwrap_or_default();

    let expanded = if let Some((_brace, items)) = mod_content {
        // Extract instruction metadata from the module
        let instructions = program::extract_instructions(items);

        // Generate MCP schema JSON
        let schema_json = mcp_gen::generate_schema_json(
            &program_name,
            &program_desc,
            &instructions,
        );

        // Generate the list_tools function and schema constant
        let list_tools = program::generate_list_tools(&schema_json);

        // Generate the instruction dispatcher
        let dispatcher = program::generate_dispatcher(mod_name, &instructions);

        // Generate the entrypoint
        let entrypoint = program::generate_entrypoint();

        quote! {
            #mod_vis mod #mod_name {
                /// MCP program name
                pub const MCP_NAME: &str = #program_name;
                /// MCP program description
                pub const MCP_DESCRIPTION: &str = #program_desc;

                #(#items)*

                // Auto-generated MCP schema and list_tools instruction
                #list_tools
            }

            // Auto-generated dispatcher and entrypoint (outside module)
            #dispatcher
            #entrypoint
        }
    } else {
        quote! { #input }
    };

    TokenStream::from(expanded)
}

/// Marks a function as an MCP tool (Solana instruction).
///
/// Generates an 8-byte discriminator using SHA256 hash of "global:<name>".
///
/// # Attributes
///
/// - `name`: Tool name (defaults to function name)
/// - `description`: Human-readable description for AI agents
///
/// # Example
///
/// ```rust,ignore
/// #[mcp_instruction(
///     name = "transfer",
///     description = "Transfer tokens from one account to another"
/// )]
/// pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
///     // Implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_instruction(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attrs = parse_instruction_attrs(attr);

    let fn_name = &input.sig.ident;

    let tool_name = attrs.name.unwrap_or_else(|| fn_name.to_string());
    let tool_desc = attrs.description.unwrap_or_default();

    // Generate SHA256-based discriminator for the instruction
    let discriminator = instruction_discriminator(&tool_name);

    // Keep the original function intact, just add metadata module
    let expanded = quote! {
        #input

        /// MCP tool metadata for this instruction
        pub mod #fn_name {
            pub const DISCRIMINATOR: [u8; 8] = [#(#discriminator),*];
            pub const TOOL_NAME: &str = #tool_name;
            pub const TOOL_DESCRIPTION: &str = #tool_desc;
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for MCP account types (resources).
///
/// Generates:
/// - `AccountDeserialize` impl using bytemuck zero-copy
/// - `AccountSerialize` impl using bytemuck zero-copy
/// - `AccountData` impl with discriminator and space
/// - `McpResource` impl for MCP schema generation
///
/// **Important**: The struct must be `#[repr(C)]` and all fields must be `Pod`-safe
/// (no padding, no references, fixed-size types only).
///
/// # Example
///
/// ```rust,ignore
/// #[derive(McpAccount, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// #[repr(C)]
/// #[mcp_account(
///     name = "user_account",
///     description = "Stores user data and balances"
/// )]
/// pub struct UserAccount {
///     pub owner: Pubkey,
///     pub balance: u64,
///     pub bump: u8,
///     pub _padding: [u8; 7],
/// }
/// ```
#[proc_macro_derive(McpAccount, attributes(mcp_account))]
pub fn derive_mcp_account(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse attributes
    let (resource_name, resource_desc) = parse_mcp_account_attrs(&input);

    // Generate SHA256-based discriminator for the account
    let discriminator = account_discriminator(&name.to_string());

    // Generate JSON schema from struct fields
    let schema_json = generate_account_schema(&input);

    let expanded = quote! {
        impl mcpsol::account::AccountDeserialize for #name {
            fn try_deserialize(data: &[u8]) -> mcpsol::Result<Self> {
                const SIZE: usize = 8 + core::mem::size_of::<#name>();

                if data.len() < SIZE {
                    return Err(mcpsol::error::McpSolError::SerializationError.into());
                }

                // Check discriminator
                let disc: [u8; 8] = [#(#discriminator),*];
                if data[..8] != disc {
                    return Err(mcpsol::error::McpSolError::InvalidAccount.into());
                }

                // Zero-copy deserialization using bytemuck
                let account_data = &data[8..SIZE];
                let account: &Self = bytemuck::from_bytes(account_data);
                Ok(*account)
            }
        }

        impl mcpsol::account::AccountSerialize for #name {
            fn try_serialize(&self, data: &mut [u8]) -> mcpsol::Result<()> {
                const SIZE: usize = 8 + core::mem::size_of::<#name>();

                if data.len() < SIZE {
                    return Err(mcpsol::error::McpSolError::SerializationError.into());
                }

                // Write discriminator
                let disc: [u8; 8] = [#(#discriminator),*];
                data[..8].copy_from_slice(&disc);

                // Zero-copy serialization using bytemuck
                let account_bytes: &[u8] = bytemuck::bytes_of(self);
                data[8..SIZE].copy_from_slice(account_bytes);

                Ok(())
            }
        }

        impl mcpsol::account::AccountData for #name {
            const DISCRIMINATOR: [u8; 8] = [#(#discriminator),*];
            const SPACE: usize = 8 + core::mem::size_of::<Self>();
        }

        impl mcpsol::traits::McpResource for #name {
            const URI_PATTERN: &'static str = "solana://{network}/account/{address}";
            const RESOURCE_NAME: &'static str = #resource_name;
            const RESOURCE_DESCRIPTION: &'static str = #resource_desc;

            fn mcp_resource_schema() -> mcpsol::mcp::McpResourceDef {
                mcpsol::mcp::McpResourceDef {
                    uri: Self::URI_PATTERN.to_string(),
                    name: Self::RESOURCE_NAME.to_string(),
                    description: Self::RESOURCE_DESCRIPTION.to_string(),
                    mime_type: "application/json".to_string(),
                    schema: mcpsol::serde_json::from_str(#schema_json).ok(),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Generate JSON schema from struct fields for MCP resource definition
fn generate_account_schema(input: &DeriveInput) -> String {
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return r#"{"type":"object"}"#.to_string(),
        },
        _ => return r#"{"type":"object"}"#.to_string(),
    };

    let mut properties = Vec::new();
    let mut required = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();

        // Skip padding fields
        if field_name.starts_with('_') {
            continue;
        }

        let field_type = type_to_json_schema(&field.ty);
        properties.push(format!(r#""{}":{}"#, field_name, field_type));
        required.push(format!(r#""{}""#, field_name));
    }

    format!(
        r#"{{"type":"object","properties":{{{}}},"required":[{}]}}"#,
        properties.join(","),
        required.join(",")
    )
}

/// Map Rust type to JSON schema type
fn type_to_json_schema(ty: &Type) -> String {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    match type_str.as_str() {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" =>
            r#"{"type":"integer","minimum":0}"#.to_string(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" =>
            r#"{"type":"integer"}"#.to_string(),
        "bool" => r#"{"type":"boolean"}"#.to_string(),
        "String" => r#"{"type":"string"}"#.to_string(),
        "Pubkey" | "pinocchio::pubkey::Pubkey" =>
            r#"{"type":"string","format":"solana-pubkey","description":"Base58-encoded public key"}"#.to_string(),
        _ if type_str.starts_with("[u8;") =>
            r#"{"type":"string","format":"base64","description":"Binary data"}"#.to_string(),
        _ => r#"{"type":"string"}"#.to_string(),
    }
}

/// Derive macro for account context structs.
///
/// Parses field attributes and generates `Accounts` trait implementation.
///
/// # Field Attributes
///
/// - `#[account(signer)]` - Verify the account is a signer
/// - `#[account(mut)]` - Verify the account is writable
/// - `#[account(owner = <program>)]` - Verify account owner
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Accounts)]
/// pub struct Initialize<'info> {
///     #[account(mut)]
///     pub counter: Account<'info, Counter>,
///     #[account(signer)]
///     pub authority: Signer<'info>,
///     pub system_program: Program<'info>,
/// }
/// ```
#[proc_macro_derive(Accounts, attributes(account))]
pub fn derive_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Accounts derive only supports named fields"),
        },
        _ => panic!("Accounts derive only supports structs"),
    };

    // Generate field extraction code
    let field_count = fields.len();
    let mut field_extractions = Vec::new();
    let mut field_names = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        field_names.push(field_name);

        // Parse #[account(...)] attributes
        let mut is_signer = false;
        let mut is_mut = false;

        for attr in &field.attrs {
            if attr.path().is_ident("account") {
                let tokens = attr.meta.require_list().ok()
                    .map(|list| list.tokens.to_string())
                    .unwrap_or_default();

                // Use word boundaries to avoid matching "cosigner" or "immutable"
                // Split on common delimiters and check for exact matches
                let parts: Vec<&str> = tokens.split(|c| c == ',' || c == ' ')
                    .map(|s| s.trim())
                    .collect();
                is_signer = parts.iter().any(|&p| p == "signer");
                is_mut = parts.iter().any(|&p| p == "mut");
            }
        }

        // Check if this is a raw reference type (starts with &)
        let ty_str = quote!(#field_ty).to_string();
        let is_raw_ref = ty_str.starts_with("&");

        // Generate extraction code based on field type and attributes
        let extraction = if is_signer {
            // Signer check - also verify writable if mut is specified
            if is_mut {
                quote! {
                    let #field_name = {
                        let info = accounts.get(#idx)
                            .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                        if !info.is_writable() {
                            return Err(mcpsol::error::McpSolError::NotWritable.into());
                        }
                        mcpsol::account::Signer::try_from(info)?
                    };
                }
            } else {
                quote! {
                    let #field_name = {
                        let info = accounts.get(#idx)
                            .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                        mcpsol::account::Signer::try_from(info)?
                    };
                }
            }
        } else if is_raw_ref {
            // Raw reference - just use the account directly
            if is_mut {
                quote! {
                    let #field_name = {
                        let info = accounts.get(#idx)
                            .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                        if !info.is_writable() {
                            return Err(mcpsol::error::McpSolError::NotWritable.into());
                        }
                        info
                    };
                }
            } else {
                quote! {
                    let #field_name = accounts.get(#idx)
                        .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                }
            }
        } else if is_mut {
            quote! {
                let #field_name = {
                    let info = accounts.get(#idx)
                        .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                    if !info.is_writable() {
                        return Err(mcpsol::error::McpSolError::NotWritable.into());
                    }
                    <#field_ty>::try_from(info)?
                };
            }
        } else {
            quote! {
                let #field_name = {
                    let info = accounts.get(#idx)
                        .ok_or(mcpsol::error::McpSolError::MissingAccount)?;
                    <#field_ty>::try_from(info)?
                };
            }
        };

        field_extractions.push(extraction);
    }

    let expanded = quote! {
        impl<'info> mcpsol::context::Accounts<'info> for #name<'info> {
            fn try_accounts(
                _program_id: &mcpsol::prelude::Pubkey,
                accounts: &'info [mcpsol::prelude::AccountInfo],
            ) -> mcpsol::Result<Self> {
                if accounts.len() < #field_count {
                    return Err(mcpsol::error::McpSolError::MissingAccount.into());
                }

                #(#field_extractions)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

// === Helper functions ===

struct ProgramAttrs {
    name: Option<String>,
    description: Option<String>,
}

fn parse_program_attrs(attr: TokenStream) -> ProgramAttrs {
    let attr_str = attr.to_string();
    ProgramAttrs {
        name: extract_attr_value(&attr_str, "name"),
        description: extract_attr_value(&attr_str, "description"),
    }
}

struct InstructionAttrs {
    name: Option<String>,
    description: Option<String>,
}

fn parse_instruction_attrs(attr: TokenStream) -> InstructionAttrs {
    let attr_str = attr.to_string();
    InstructionAttrs {
        name: extract_attr_value(&attr_str, "name"),
        description: extract_attr_value(&attr_str, "description"),
    }
}

fn parse_mcp_account_attrs(input: &DeriveInput) -> (String, String) {
    let mut name = input.ident.to_string();
    let mut desc = String::new();

    // Parse #[mcp_account(...)] attributes
    for attr in &input.attrs {
        if attr.path().is_ident("mcp_account") {
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();
                if let Some(n) = extract_attr_value(&tokens, "name") {
                    name = n;
                }
                if let Some(d) = extract_attr_value(&tokens, "description") {
                    desc = d;
                }
            }
        }
    }

    (name, desc)
}

fn extract_attr_value(attr_str: &str, key: &str) -> Option<String> {
    let pattern = format!("{} = \"", key);
    if let Some(start) = attr_str.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = attr_str[value_start..].find('"') {
            return Some(attr_str[value_start..value_start + end].to_string());
        }
    }
    None
}
