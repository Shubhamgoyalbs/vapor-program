use anchor_lang::prelude::*;
use crate::constants::{MAX_ID_LEN, MAX_PAYER_ALLOWLIST, MAX_SPLIT_RECIPIENTS};
use crate::state::seeds::PAYMENT_ORDER_SEED;

/// One per client-supplied order id, scoped to an api_holder
/// (seeds: [PAYMENT_ORDER_SEED, api_holder, id]). Never closed on
/// completion — `is_closed` / `payers_paid` carry final state; rent is
/// negligible here.
///
/// This is a reusable payment LINK, not a single-use invoice: up to
/// `payer_cap` distinct wallets may each pay `total_amount` in full,
/// independently. `vault` is a single POOLED escrow shared by every
/// payer's refundable contribution — created unconditionally here, at
/// order-creation time, whether or not the order ends up refundable
/// (same "always create it, sometimes it's just a pass-through"
/// resolution used elsewhere in this program to avoid conditional
/// account creation). Each payer's own slice of that pool, and whether
/// it's been resolved, is tracked on their own `OrderPayment`.
#[account]
#[derive(InitSpace)]
pub struct PaymentOrder {
  #[max_len(MAX_ID_LEN)]
  pub id: String,
  pub api_holder: Pubkey,
  pub mint: Pubkey,
  /// Amount EACH payer pays, in full. includes program + api_holder fee.
  pub total_amount: u64,
  /// How each individual payment splits across recipients. All-None =
  /// a single default recipient the pay/settle instruction resolves
  /// itself.
  #[max_len(MAX_SPLIT_RECIPIENTS)]
  pub split_recipients: Vec<SplitRecipient>,
  /// All-None = open to anyone (bounded only by `payer_cap`, if
  /// nonzero). Non-empty = only these wallets may ever pay, one
  /// payment each.
  #[max_len(MAX_PAYER_ALLOWLIST)]
  pub allowed_payers: Vec<Pubkey>,
  /// Max distinct payers who may ever pay this order. 0 = uncapped.
  /// If `allowed_payers` is non-empty, this is clamped at creation to
  /// its populated length.
  pub payer_cap: u16,
  /// Running count of distinct payers who have completed payment.
  pub payers_paid: u16,
  /// Maker-controlled early-close switch, set via
  /// `close_payment_order` — independent of `payer_cap` and `expiry`.
  pub is_closed: bool,
  /// No new payments accepted after this unix timestamp. None = no
  /// expiry.
  pub expiry: Option<i64>,
  /// Portion of each individual payment that's refundable, in
  /// PERCENTAGE_PRECISION units. 0 = every payment settles instantly
  /// at pay-time, no escrow involved at all.
  pub refundable_percentage: u32,
  /// Refund window length in seconds from EACH payer's own `paid_at`
  /// — not a single order-wide deadline. Always `None` when
  /// `refundable_percentage` is 0, always `Some` otherwise.
  pub refund_window_seconds: Option<i64>,
  /// Pooled escrow holding every payer's still-pending refundable
  /// slice until `settle_order` (window closed, sweep to recipients)
  /// or `refund_order` (maker refunds one payer within their window)
  /// resolves it.
  pub vault_bump: u8,
  pub created_at: i64,
  pub bump: u8,
  pub _reserved: [u8; 16],
}

impl PaymentOrder {
  pub const SEED_PREFIX: &'static [u8] = PAYMENT_ORDER_SEED;
  
  /// True for an order created via `create_payment_order` (the SOL
  /// rail) — see NATIVE_SOL_MINT_SENTINEL. Token-rail orders go
  /// through `create_payment_order_token`.
  pub fn is_native_sol(&self) -> bool {
    self.mint == crate::constants::NATIVE_SOL_MINT_SENTINEL
  }
  
  /// True if `payer` is allowed to pay this order right now — checks
  /// the allowlist only; caller still separately checks `is_closed`,
  /// `expiry`, and `payer_cap`.
  pub fn payer_is_eligible(&self, payer: &Pubkey) -> bool {
    self.allowed_payers.is_empty() || self.allowed_payers.contains(payer)
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq)]
pub struct SplitRecipient {
  pub recipient: Pubkey,
  /// PERCENTAGE_PRECISION units. Active (Some) entries must sum to
  /// exactly PERCENTAGE_PRECISION — enforced at order creation, not
  /// re-checked here.
  pub percentage: u32,
}