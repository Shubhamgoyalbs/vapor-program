use anchor_lang::prelude::*;

use crate::constants::{MAX_ID_LEN, NATIVE_SOL_MINT_SENTINEL};
use crate::state::seeds::{SUBSCRIPTION_POLICY_SEED, SUBSCRIPTION_SEED};

/// Terms for recurring charges. One per client-supplied policy id,
/// scoped to an api_holder (seeds: [SUBSCRIPTION_POLICY_SEED, api_holder, id]).
///
/// Charges are pulled from an escrow vault the subscriber prefunds
/// (`Subscription.vault`), not an SPL delegate — see `Subscription`'s
/// doc comment for why.
#[account]
#[derive(InitSpace)]
pub struct SubscriptionPolicy {
  #[max_len(MAX_ID_LEN)]
  pub id: String,
  pub api_holder: Pubkey,
  pub mint: Pubkey,
  /// Destination wallet for every charge under this policy — may
  /// differ from api_holder.owner (e.g. a dedicated treasury).
  pub recipient: Pubkey,
  pub amount_per_charge: u64,
  /// Seconds between charges.
  pub charge_interval: i64,
  /// 0 = unlimited charges.
  pub max_charges: u32,
  /// Consecutive failed-charge retries allowed (insufficient escrow
  /// balance) before a subscription lapses. Unlike an SPL-delegate
  /// design, this is checked and handled entirely inside
  /// `charge_subscription` itself — see that instruction's doc
  /// comment for why escrow makes that safe to do in one transaction.
  pub retry_count: u8,
  pub created_at: i64,
  pub bump: u8,
  pub _reserved: [u8; 16],
}

impl SubscriptionPolicy {
  pub const SEED_PREFIX: &'static [u8] = SUBSCRIPTION_POLICY_SEED;
  
  pub fn is_native_sol(&self) -> bool {
    self.mint == NATIVE_SOL_MINT_SENTINEL
  }
}

/// One subscriber's enrollment under a policy
/// (seeds: [SUBSCRIPTION_SEED, policy, subscriber]).
///
/// `vault` is this subscriber's own prefunded escrow for this specific
/// subscription — not a shared pool, not a delegate against their main
/// wallet. They (or anyone, via `top_up_subscription`) deposit into it
/// ahead of billing; `charge_subscription` debits it directly, signed
/// by this account's own PDA seeds (same pattern as `OrderPayment`'s
/// vault). Running dry doesn't touch their wallet balance at all — it
/// just means retries, then a lapse, against funds already set aside.
#[account]
#[derive(InitSpace)]
pub struct Subscription {
  pub policy: Pubkey,
  pub subscriber: Pubkey,
  pub vault: Pubkey,
  pub vault_bump: u8,
  pub charges_made: u32,
  /// Consecutive failed attempts (escrow balance was too low) against
  /// the policy's `retry_count`. Reset to 0 on a successful charge.
  pub retries_used: u8,
  pub next_charge_at: i64,
  pub status: SubscriptionStatus,
  pub bump: u8,
}

impl Subscription {
  pub const SEED_PREFIX: &'static [u8] = SUBSCRIPTION_SEED;
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionStatus {
  Active,
  /// Subscriber-initiated; any remaining escrow is refunded in the
  /// same instruction.
  Cancelled,
  /// max_charges reached; any remaining escrow is refunded in the
  /// same instruction that completes it.
  Completed,
  /// retry_count exhausted on insufficient escrow balance; whatever
  /// was left (less than one full charge, by definition) is refunded
  /// in the same instruction that lapses it.
  Lapsed,
}