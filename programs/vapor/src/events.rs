use anchor_lang::prelude::*;

/// Emitted on every state-changing instruction that collects a fee
/// (order settlement, subscription charge, vesting claim, ...). One
/// shared shape instead of a near-duplicate per domain, per the spec's
/// "unified events" rule.
#[event]
pub struct FeeCharged {
  pub api_holder: Pubkey,
  /// The order / subscription / vesting-policy this fee came from.
  pub source: Pubkey,
  /// Fees can arrive in a token other than native SOL (the holder's
  /// vault currency) —
  /// always capture which one, not just the amount.
  pub mint: Pubkey,
  pub program_fee_amount: u64,
  pub api_fee_amount: u64,
}

// -- init_api_holder --

#[event]
pub struct ApiHolderInitialized {
  pub api_holder: Pubkey,
  pub auth: Pubkey,
  pub owner: Pubkey,
  pub fee_percentage: u32,
  pub is_active: bool,
}

// -- deposit / withdraw --

#[event]
pub struct Deposited {
  pub api_holder: Pubkey,
  pub amount: u64,
  pub new_vault_balance: u64,
}

#[event]
pub struct Withdrawn {
  pub api_holder: Pubkey,
  pub amount: u64,
  pub new_vault_balance: u64,
}

#[event]
pub struct TokenWithdrawn {
  pub api_holder: Pubkey,
  pub mint: Pubkey,
  pub amount: u64,
  pub destination: Pubkey,
}

// -- create_payment_order / close_payment_order --

#[event]
pub struct PaymentOrderCreated {
  pub order: Pubkey,
  pub api_holder: Pubkey,
  pub mint: Pubkey,
  pub total_amount: u64,
  pub payer_cap: u16,
  pub expiry: Option<i64>,
  pub refundable_percentage: u32,
  pub refund_window_seconds: Option<i64>,
  pub vault: Pubkey,
}

#[event]
pub struct PaymentOrderClosed {
  pub order: Pubkey,
}

// -- pay_order / settle_order / refund_order --

#[event]
pub struct OrderPaid {
  pub order: Pubkey,
  pub order_payment: Pubkey,
  pub payer: Pubkey,
  pub amount: u64,
  /// This payer's own slice that went into the pool, if any — 0 and
  /// `settled_immediately: true` means this payment was non-refundable
  /// and already fully distributed inside `pay_order` itself.
  pub refundable_amount: u64,
  pub settled_immediately: bool,
  pub paid_at: i64,
  pub refund_valid_until: Option<i64>,
}

#[event]
pub struct OrderSettled {
  pub order: Pubkey,
  pub order_payment: Pubkey,
  pub amount: u64,
}

#[event]
pub struct OrderRefunded {
  pub order: Pubkey,
  pub order_payment: Pubkey,
  pub payer: Pubkey,
  pub refunded_amount: u64,
  pub settled_amount: u64,
}

// -- create_subscription_policy / subscribe / charge_subscription /
// report_failed_charge / cancel_subscription --

#[event]
pub struct SubscriptionPolicyCreated {
  pub policy: Pubkey,
  pub api_holder: Pubkey,
  pub mint: Pubkey,
  pub recipient: Pubkey,
  pub amount_per_charge: u64,
  pub charge_interval: i64,
  pub max_charges: u32,
  pub retry_count: u8,
}

#[event]
pub struct Subscribed {
  pub subscription: Pubkey,
  pub policy: Pubkey,
  pub subscriber: Pubkey,
  pub initial_deposit: u64,
}

#[event]
pub struct SubscriptionToppedUp {
  pub subscription: Pubkey,
  pub amount: u64,
  pub new_vault_balance: u64,
}

#[event]
pub struct SubscriptionCharged {
  pub subscription: Pubkey,
  pub policy: Pubkey,
  pub amount: u64,
  pub charges_made: u32,
  pub next_charge_at: i64,
  /// Set when this charge was the last one (max_charges reached) and
  /// any escrow left over was refunded in the same instruction.
  pub completed_refund: Option<u64>,
}

#[event]
pub struct ChargeFailureRecorded {
  pub subscription: Pubkey,
  pub retries_used: u8,
}

#[event]
pub struct SubscriptionLapsed {
  pub subscription: Pubkey,
  /// Whatever was left in escrow (less than one charge, by
  /// definition) — refunded to the subscriber in the same instruction.
  pub refunded_amount: u64,
}

#[event]
pub struct SubscriptionCancelled {
  pub subscription: Pubkey,
  pub refunded_amount: u64,
}

// -- create_vesting_policy / invite / accept / claim / cancel --

#[event]
pub struct VestingPolicyCreated {
  pub policy: Pubkey,
  pub api_holder: Pubkey,
  pub maker: Pubkey,
  pub mint: Pubkey,
  pub total_amount: u64,
  pub cancellable: bool,
}

#[event]
pub struct VestingEnrollmentInvited {
  pub enrollment: Pubkey,
  pub policy: Pubkey,
  pub invitee: Pubkey,
}

#[event]
pub struct VestingEnrollmentAccepted {
  pub enrollment: Pubkey,
  pub accepted_at: i64,
}

#[event]
pub struct VestingTrancheClaimed {
  pub enrollment: Pubkey,
  pub policy: Pubkey,
  pub amount: u64,
  pub tranches_claimed: u8,
  pub next_tranche_index: u8,
}

#[event]
pub struct VestingEnrollmentCancelled {
  pub enrollment: Pubkey,
  pub swept_to_maker: u64,
  pub claim_ceiling: u8,
}

#[event]
pub struct VestingEnrollmentSwept {
  pub enrollment: Pubkey,
  pub policy: Pubkey,
  pub invitee: Pubkey,
  pub swept_to_maker: u64,
}

// -- activate_api_holder / deactivate_api_holder --

#[event]
pub struct ApiHolderActivated {
  pub api_holder: Pubkey,
}

#[event]
pub struct ApiHolderDeactivated {
  pub api_holder: Pubkey,
}