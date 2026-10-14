use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::state::seeds::API_HOLDER_SEED;

/// One per `owner` wallet (seeds: [API_HOLDER_SEED, owner]).
///
/// The vault is a system-owned account holding native SOL only — base
/// currency is always SOL, not a configurable field. The vault holds
/// this holder's own working capital (rent/gas float, withdrawable
/// earned fees) — it is NOT where customer payment proceeds sit; those
/// go straight into each order's own escrow vault at pay-time.
#[account]
#[derive(InitSpace)]
pub struct ApiHolder {
  /// The Privy-managed embedded wallet allowed to request transaction
  /// creation (orders, policies, charges) on this holder's behalf.
  /// Already exists off-chain by the time this account is created —
  /// no on-chain activation step needed, unlike an MPC provider (e.g.
  /// Turnkey) where key provisioning is async.
  pub auth: Pubkey,
  /// Allowed to deposit and withdraw funds. Only signer accepted on
  /// `deposit` / `withdraw`.
  pub owner: Pubkey,
  /// Bump for this account's own PDA.
  pub bump: u8,
  /// System-owned account derived from this PDA. Holds native SOL.
  pub vault: Pubkey,
  /// Bump for `vault`. Stored so later instructions can re-derive the
  /// vault's signer seeds for a CPI without recomputing
  /// `find_program_address`.
  pub vault_bump: u8,
  /// Explicit owner-controlled switch — set at init, then flipped only
  /// by `activate_api_holder` / `deactivate_api_holder`. Not derived
  /// from vault balance; `withdraw` enforces its own MIN_VAULT_LAMPORTS
  /// floor separately and independently of this flag.
  pub is_active: bool,
  /// This holder's fee, in PERCENTAGE_PRECISION units, layered on top
  /// of PROGRAM_FEE_PERCENTAGE. Set once at init.
  pub fee_percentage: u32,
  pub created_at: i64,
  pub _reserved: [u8; 32],
}

impl ApiHolder {
  pub const SEED_PREFIX: &'static [u8] = API_HOLDER_SEED;
  
  /// Call only from instructions that actually require the auth key —
  /// e.g. create_order, create_subscription_policy. Do NOT call this
  /// from payer-facing or permissionless instructions (pay_order,
  /// claim_vesting_tranche, charge_subscription) — those authorize
  /// differently (payer signature, or no signature at all).
  pub fn require_auth(&self, signer: &Pubkey) -> Result<()> {
    require!(self.is_active, ErrorCode::ApiHolderInactive);
    require_keys_eq!(self.auth, *signer, ErrorCode::UnauthorizedAuth);
    Ok(())
  }
  
  /// Call from `deposit` / `withdraw` — the only instructions that
  /// should ever check `owner` rather than `auth`.
  pub fn require_owner(&self, signer: &Pubkey) -> Result<()> {
    require_keys_eq!(self.owner, *signer, ErrorCode::UnauthorizedOwner);
    Ok(())
  }
}