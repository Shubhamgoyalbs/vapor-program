pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod events;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("946VHDUXU1srWsYRGxnYxAzHvBKxgFEmKvQotcTUBuYW");

#[program]
pub mod vapor {
  use super::*;
  
  /// Api Holder
  
  pub fn init_api_holder(
    ctx: Context<InitApiHolder>,
    fee_percentage: u32,
  ) -> Result<()> {
    init_api_holder::handler(ctx, fee_percentage)
  }
  
  pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    deposit::handler(ctx, amount)
  }
  
  pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    withdraw::handler(ctx, amount)
  }
  
  pub fn activate_api_holder(ctx: Context<ActivateApiHolder>) -> Result<()> {
    activate_api_holder::handler(ctx)
  }
  
  pub fn deactivate_api_holder(ctx: Context<DeactivateApiHolder>) -> Result<()> {
    deactivate_api_holder::handler(ctx)
  }
  
  pub fn request_auth_fund(ctx: Context<RequestAuthFund>) -> Result<()> {
    request_auth_fund::handler(ctx)
  }

  pub fn sweep_treasury(ctx: Context<SweepTreasury>, amount: u64) -> Result<()> {
    instructions::sweep_treasury::handler(ctx, amount)
  }

  pub fn sweep_treasury_token(ctx: Context<SweepTreasuryToken>, amount: u64) -> Result<()> {
    instructions::sweep_treasury_token::handler(ctx, amount)
  }

  /// Payment Order
  
  pub fn create_payment_order(
    ctx: Context<CreatePaymentOrder>,
    id: String,
    total_amount: u64,
    split_recipients: Vec<state::SplitRecipient>,
    allowed_payers: Vec<Pubkey>,
    payer_cap: u16,
    expiry: Option<i64>,
    refundable_percentage: u32,
    refund_window_seconds: Option<i64>,
  ) -> Result<()> {
    create_payment_order::handler(
      ctx,
      id,
      total_amount,
      split_recipients,
      allowed_payers,
      payer_cap,
      expiry,
      refundable_percentage,
      refund_window_seconds,
    )
  }
  
  pub fn create_payment_order_token(
    ctx: Context<CreatePaymentOrderToken>,
    id: String,
    total_amount: u64,
    split_recipients: Vec<state::SplitRecipient>,
    allowed_payers: Vec<Pubkey>,
    payer_cap: u16,
    expiry: Option<i64>,
    refundable_percentage: u32,
    refund_window_seconds: Option<i64>,
  ) -> Result<()> {
    create_payment_order_token::handler(
      ctx,
      id,
      total_amount,
      split_recipients,
      allowed_payers,
      payer_cap,
      expiry,
      refundable_percentage,
      refund_window_seconds,
    )
  }

  pub fn close_payment_order(ctx: Context<ClosePaymentOrder>) -> Result<()> {
    close_payment_order::handler(ctx)
  }
  
  pub fn pay_order_token<'info>(ctx: Context<'info, PayOrderToken<'info>>) -> Result<()> {
    pay_order_token::handler(ctx)
  }
  
  pub fn settle_order_token<'info>(ctx: Context<'info, SettleOrderToken<'info>>) -> Result<()> {
    settle_order_token::handler(ctx)
  }
  
  pub fn refund_order_token<'info>(ctx: Context<'info, RefundOrderToken<'info>>) -> Result<()> {
    refund_order_token::handler(ctx)
  }
  
  pub fn pay_order<'info>(ctx: Context<'info, PayOrder<'info>>) -> Result<()> {
    pay_order::handler(ctx)
  }
  
  pub fn settle_order<'info>(ctx: Context<'info, SettleOrder<'info>>) -> Result<()> {
    settle_order::handler(ctx)
  }
  
  pub fn refund_order<'info>(ctx: Context<'info, RefundOrder<'info>>) -> Result<()> {
    refund_order::handler(ctx)
  }
  
  /// Vesting
  
  pub fn create_vesting_policy_token(
    ctx: Context<CreateVestingPolicyToken>,
    id: String,
    tranches: Vec<state::Tranche>,
    cancellable: bool,
    cancellable_after_tranche_index: Option<u8>,
    min_hard_locked_amount: u64,
  ) -> Result<()> {
    instructions::create_vesting_policy_token::handler(
      ctx,
      id,
      tranches,
      cancellable,
      cancellable_after_tranche_index,
      min_hard_locked_amount,
    )
  }
  
  pub fn create_vesting_policy(
    ctx: Context<CreateVestingPolicy>,
    id: String,
    tranches: Vec<state::Tranche>,
    cancellable: bool,
    cancellable_after_tranche_index: Option<u8>,
    min_hard_locked_amount: u64,
  ) -> Result<()> {
    instructions::create_vesting_policy::handler(
      ctx,
      id,
      tranches,
      cancellable,
      cancellable_after_tranche_index,
      min_hard_locked_amount,
    )
  }
  
  pub fn invite_vesting_enrollment(
    ctx: Context<InviteVestingEnrollment>,
    invitee: Pubkey,
  ) -> Result<()> {
    instructions::invite_vesting_enrollment::handler(ctx, invitee)
  }
  
  pub fn invite_vesting_enrollment_token(
    ctx: Context<InviteVestingEnrollmentToken>,
    invitee: Pubkey,
  ) -> Result<()> {
    instructions::invite_vesting_enrollment_token::handler(ctx, invitee)
  }
  
  pub fn accept_vesting_enrollment(ctx: Context<AcceptVestingEnrollment>) -> Result<()> {
    instructions::accept_vesting_enrollment::handler(ctx)
  }
  
  pub fn claim_vesting_tranche_token(ctx: Context<ClaimVestingTrancheToken>) -> Result<()> {
    instructions::claim_vesting_tranche_token::handler(ctx)
  }
  
  pub fn claim_vesting_tranche(ctx: Context<ClaimVestingTranche>) -> Result<()> {
    instructions::claim_vesting_tranche::handler(ctx)
  }
  
  pub fn cancel_vesting_enrollment_token(ctx: Context<CancelVestingEnrollmentToken>) -> Result<()> {
    instructions::cancel_vesting_enrollment_token::handler(ctx)
  }
  
  pub fn cancel_vesting_enrollment(ctx: Context<CancelVestingEnrollment>) -> Result<()> {
    instructions::cancel_vesting_enrollment::handler(ctx)
  }

  pub fn sweep_unaccepted_invite(ctx: Context<SweepUnacceptedInvite>) -> Result<()> {
    instructions::sweep_unaccepted_invite::handler(ctx)
  }

  pub fn sweep_unaccepted_invite_token(ctx: Context<SweepUnacceptedInviteToken>) -> Result<()> {
    instructions::sweep_unaccepted_invite_token::handler(ctx)
  }

  /// Subscription
  
  pub fn create_subscription_policy(
    ctx: Context<CreateSubscriptionPolicy>,
    id: String,
    recipient: Pubkey,
    amount_per_charge: u64,
    charge_interval: i64,
    max_charges: u32,
    retry_count: u8,
  ) -> Result<()> {
    instructions::create_subscription_policy::handler(
      ctx,
      id,
      recipient,
      amount_per_charge,
      charge_interval,
      max_charges,
      retry_count,
    )
  }
  
  pub fn subscribe(ctx: Context<Subscribe>, initial_deposit: u64) -> Result<()> {
    instructions::subscribe::handler(ctx, initial_deposit)
  }
  
  pub fn subscribe_token(ctx: Context<SubscribeToken>, initial_deposit: u64) -> Result<()> {
    instructions::subscribe_token::handler(ctx, initial_deposit)
  }
  
  pub fn top_up_subscription(ctx: Context<TopUpSubscription>, amount: u64) -> Result<()> {
    instructions::top_up_subscription::handler(ctx, amount)
  }
  
  pub fn top_up_subscription_token(ctx: Context<TopUpSubscriptionToken>, amount: u64) -> Result<()> {
    instructions::top_up_subscription_token::handler(ctx, amount)
  }
  
  pub fn charge_subscription(ctx: Context<ChargeSubscription>) -> Result<()> {
    instructions::charge_subscription::handler(ctx)
  }
  
  pub fn charge_subscription_token(ctx: Context<ChargeSubscriptionToken>) -> Result<()> {
    instructions::charge_subscription_token::handler(ctx)
  }
  
  pub fn cancel_subscription(ctx: Context<CancelSubscription>) -> Result<()> {
    instructions::cancel_subscription::handler(ctx)
  }
  
  pub fn cancel_subscription_token(ctx: Context<CancelSubscriptionToken>) -> Result<()> {
    instructions::cancel_subscription_token::handler(ctx)
  }
}
