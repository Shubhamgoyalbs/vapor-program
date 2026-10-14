use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::PaymentOrderClosed;
use crate::state::{ApiHolder, PaymentOrder};

/// Stops NEW payments only. Doesn't touch any payer's existing
/// OrderPayment — settle / refund keep working on already-paid
/// payments regardless of whether the order itself is still open.
pub fn handler(ctx: Context<ClosePaymentOrder>) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  let order = &mut ctx.accounts.order;
  require!(!order.is_closed, ErrorCode::OrderClosed);
  order.is_closed = true;
  
  emit_cpi!(PaymentOrderClosed { order: order.key() });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct ClosePaymentOrder<'info> {
  pub auth: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  #[account(
        mut,
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), order.id.as_bytes()],
        bump = order.bump,
  )]
  pub order: Account<'info, PaymentOrder>,
}