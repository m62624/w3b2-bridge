// src/onchain_client.rs

use crate::keystore::ChainCard;
use anchor_client::{
    solana_sdk::{pubkey::Pubkey, signature::Signature, signer::Signer, system_program},
    Client, Cluster, Program,
};
use anchor_lang::declare_program;
use std::{rc::Rc, sync::Arc};
// This macro reads the IDL from the `/idls` directory and generates a client module.
// The name `w3b2_bridge_program_client` is arbitrary.
declare_program!(w3b2_bridge_program);
// We can now use the auto-generated types for accounts and args.
