//! Centralized PDA seed prefixes. Never inline byte literals for seeds
//! anywhere else in the program — import from here so a seed change is
//! always a one-file edit.

pub const TREASURY_SEED: &[u8] = b"treasury";

pub const API_HOLDER_SEED: &[u8] = b"api_holder";
pub const API_HOLDER_VAULT_SEED: &[u8] = b"api_holder_vault";

pub const PAYMENT_ORDER_SEED: &[u8] = b"order";
pub const PAYMENT_ORDER_VAULT_SEED: &[u8] = b"order_vault";

pub const ORDER_PAYMENT_SEED: &[u8] = b"order_payment";
pub const ORDER_PAYMENT_VAULT_SEED: &[u8] = b"order_payment_vault";

pub const SUBSCRIPTION_POLICY_SEED: &[u8] = b"sub_policy";
pub const SUBSCRIPTION_SEED: &[u8] = b"subscription";
pub const SUBSCRIPTION_VAULT_SEED: &[u8] = b"subscription_vault";

pub const VESTING_POLICY_SEED: &[u8] = b"vesting_policy";
pub const VESTING_VAULT_SEED: &[u8] = b"vesting_vault";
pub const VESTING_ENROLLMENT_SEED: &[u8] = b"vesting_enrollment";