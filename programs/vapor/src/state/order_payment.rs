use anchor_lang::prelude::*;

use crate::state::seeds::ORDER_PAYMENT_SEED;

/// One per (order, payer) — created by `pay_order` / `pay_order_token`,
/// never by `create_payment_order`, since a single `PaymentOrder` can
/// have up to `payer_cap` of these
/// (seeds: [ORDER_PAYMENT_SEED, order, payer]). Anchor's own `init`
/// constraint on this PDA is what actually prevents the same payer from
/// paying the same order twice.
///
/// Pure bookkeeping — holds no vault of its own. The money sits in the
/// order's single pooled `PaymentOrder.vault`; this just tracks THIS
/// payer's own slice of it (`refundable_amount`) and whether that slice
/// has been resolved, so `refund_order` knows exactly how much to send
/// back when the maker refunds one specific payer out of the shared pool.
#[account]
#[derive(InitSpace)]
pub struct OrderPayment {
  pub order: Pubkey,
  pub payer: Pubkey,
  pub paid_at: i64,
  /// This payer's own contribution to the refund pool —
  /// `total_amount * refundable_percentage / PERCENTAGE_PRECISION` at
  /// the moment of payment. 0 if the order wasn't refundable at
  /// pay-time (in which case this payment was settled directly, in
  /// full, inside `pay_order` itself — see that instruction).
  pub refundable_amount: u64,
  /// `paid_at + PaymentOrder.refund_window_seconds` at the moment of
  /// payment — fixed here rather than recomputed later, since the
  /// order's own refund terms could in principle change after this
  /// payment was made. None if the order wasn't refundable at pay-time.
  pub refund_valid_until: Option<i64>,
  /// Set once this payer's full amount has left the pool — either
  /// settled immediately at pay-time (non-refundable order), swept by
  /// `settle_order` once the window closed, or resolved by
  /// `refund_order`.
  pub settled: bool,
  pub bump: u8,
}

impl OrderPayment {
  pub const SEED_PREFIX: &'static [u8] = ORDER_PAYMENT_SEED;
}