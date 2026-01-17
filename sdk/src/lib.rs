//! # mcpsol - MCP-native Solana SDK
//!
//! Replace IDL with Model Context Protocol for Solana programs.
//! Built on Pinocchio for minimal compute overhead.
//!
//! ## Core Concepts
//!
//! - **MCP Tools** = Solana Instructions
//! - **MCP Tool Parameters** = Instruction args + accounts
//! - **MCP Resources** = On-chain account data
//!
//! ## Example
//!
//! ```rust,ignore
//! use mcpsol::prelude::*;
//!
//! #[mcp_program]
//! pub mod my_program {
//!     #[mcp_instruction(
//!         name = "transfer_tokens",
//!         description = "Transfer SPL tokens between accounts"
//!     )]
//!     pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
//!         // ...
//!     }
//! }
//! ```

pub mod account;
pub mod context;
pub mod error;
pub mod mcp;
pub mod traits;

pub mod prelude {
    pub use crate::account::*;
    pub use crate::context::*;
    pub use crate::error::{McpSolError, Result};
    pub use crate::mcp::*;
    pub use crate::traits::*;
    pub use mcpsol_macros::*;
    pub use pinocchio::account_info::AccountInfo;
    pub use pinocchio::entrypoint;
    pub use pinocchio::program_error::ProgramError;
    pub use pinocchio::pubkey::Pubkey;
}

/// Re-export pinocchio for convenience
pub use pinocchio;
