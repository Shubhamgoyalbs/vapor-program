use anchor_lang::prelude::*;

/// Grows one instruction at a time — only variants a currently-built
/// instruction actually raises live here. No catch-all "InvalidInput".
#[error_code]
pub enum ErrorCode {
  #[msg("order/policy id exceeds 32 bytes — strip UUID hyphens or use CUID2")]
  IdTooLong,
  #[msg("invalid token account data (minimum 165 bytes required)")]
  InvalidTokenAccount,
  #[msg("token account owner does not match expected authority")]
  TokenAccountOwnerMismatch,
  #[msg("not enough remaining_accounts supplied for split_recipients")]
  MissingRecipientAccounts,
  #[msg("a supplied recipient token account doesn't match the expected ATA")]
  RecipientAccountMismatch,

  // -- ApiHolder --
  #[msg("fee_percentage + PROGRAM_FEE_PERCENTAGE must not exceed 100_000_000")]
  FeePercentageTooHigh,
  #[msg("api holder is inactive or has no auth key set")]
  ApiHolderInactive,
  #[msg("signer is not this api holder's auth key")]
  UnauthorizedAuth,
  #[msg("signer is not this api holder's owner key")]
  UnauthorizedOwner,
  #[msg("signer is not the treasury admin")]
  UnauthorizedAdmin,
  #[msg("withdrawal would drop the vault below MIN_VAULT_LAMPORTS")]
  BelowMinBalance,
  #[msg("api holder is already active")]
  AlreadyActive,
  #[msg("api holder is already inactive")]
  AlreadyInactive,
  #[msg("token vault balance is less than the requested withdrawal amount")]
  InsufficientVaultTokenBalance,
  #[msg("vault balance is less than the required amount")]
  InsufficientVaultFunds,
  
  // -- PaymentOrder --
  #[msg("split_recipients percentages must sum to exactly 100_000_000")]
  SplitPercentageMismatch,
  #[msg("order has expired")]
  OrderExpired,
  #[msg("order already paid")]
  OrderAlreadyPaid,
  #[msg("signer is not this order's designated payer")]
  UnauthorizedPayer,
  #[msg("refundable_percentage must not exceed 100_000_000")]
  InvalidRefundablePercentage,
  #[msg("refund_valid_until must be set and in the future iff refundable_percentage > 0")]
  InvalidRefundWindow,
  #[msg("mint does not match the order's mint")]
  OrderMintMismatch,
  #[msg("order is closed to further payment")]
  OrderClosed,
  #[msg("payer_cap reached for this order")]
  PayerCapReached,
  #[msg("this order has already been settled")]
  OrderAlreadySettled,
  #[msg("refund window has not yet closed")]
  RefundWindowStillOpen,
  #[msg("refund window has closed")]
  RefundWindowExpired,
  #[msg("this order is not refundable")]
  OrderNotRefundable,
  #[msg("split_recipients cannot be empty")]
  EmptySplitRecipients,
  #[msg("too many split_recipients")]
  TooManySplitRecipients,
  #[msg("split percentage must be greater than zero")]
  ZeroSplitPercentage,
  #[msg("duplicate split recipient found")]
  DuplicateSplitRecipient,
  #[msg("too many allowed_payers")]
  TooManyAllowedPayers,
  #[msg("duplicate allowed_payer found")]
  DuplicateAllowedPayer,
  #[msg("too many tranches")]
  TooManyTranches,
  
  // -- SubscriptionPolicy / Subscription --
  #[msg("charge_interval must be greater than zero")]
  InvalidChargeInterval,
  #[msg("retry_count must not exceed 10")]
  InvalidRetryCount,
  #[msg("subscription is not active")]
  SubscriptionNotActive,
  #[msg("next charge is not due yet")]
  ChargeNotDue,
  #[msg("mint does not match the subscription policy's mint")]
  SubscriptionMintMismatch,
  
  // -- VestingPolicy / VestingEnrollment --
  #[msg("tranches must be non-empty, strictly increasing by unlock_time, and every amount > 0")]
  InvalidTrancheSchedule,
  #[msg("cancellable_after_tranche_index is out of range, or set while cancellable is false")]
  InvalidCancellationTerms,
  #[msg("min_hard_locked_amount cannot exceed total_amount")]
  InvalidHardLockedAmount,
  #[msg("mint does not match the vesting policy's mint")]
  VestingMintMismatch,
  #[msg("this policy already has an invitee — cancelled policies are never reused")]
  AlreadyEnrolled,
  #[msg("signer is not this policy's maker")]
  UnauthorizedMaker,
  #[msg("signer is not this enrollment's invitee")]
  UnauthorizedInvitee,
  #[msg("enrollment is not awaiting acceptance")]
  EnrollmentNotInvited,
  #[msg("enrollment has no claimable tranches right now")]
  EnrollmentNotClaimable,
  #[msg("no tranche has newly vested since the last claim")]
  NothingVestedYet,
  #[msg("this vesting policy is not cancellable")]
  VestingNotCancellable,
  #[msg("cancellation gate not yet met (tranche index and/or min_hard_locked_amount)")]
  CancellationGateNotMet,
  #[msg("enrollment is not in the expected status")]
  InvalidEnrollmentStatus,
  #[msg("invite has not yet expired (INVITE_EXPIRY_SECONDS not reached)")]
  InviteNotExpired,
}