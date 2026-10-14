use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::OrderRefunded;
use crate::helper::distribute;
use crate::state::seeds::{
  API_HOLDER_VAULT_SEED, ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED,
};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Maker-initiated (`auth`-gated), not payer-initiated: the order's
/// maker refunds ONE specific payer, while that payer's own window is
/// still open. Sends back that payer's `refundable_amount` from the
/// pooled vault, and settles the rest of THEIR contribution
/// (`total_amount - refundable_amount`) to recipients in the same call
/// — nothing left to wait for once a refund's been issued.
pub fn handler<'info>(ctx: Context<'info, RefundOrder<'info>>) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  let order = &ctx.accounts.order;
  let order_payment = &ctx.accounts.order_payment;
  
  require!(order.is_native_sol(), ErrorCode::OrderMintMismatch);
  require!(!order_payment.settled, ErrorCode::OrderAlreadySettled);
  require!(order.refundable_percentage > 0, ErrorCode::OrderNotRefundable);
  let deadline = order_payment
    .refund_valid_until
    .ok_or(ErrorCode::OrderNotRefundable)?;
  require!(
        Clock::get()?.unix_timestamp <= deadline,
        ErrorCode::RefundWindowExpired
    );
  
  let refundable_amount = order_payment.refundable_amount;
  let non_refundable_amount = order.total_amount.saturating_sub(refundable_amount);
  
  let order_key = order.key();
  let vault_bump = order.vault_bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[PAYMENT_ORDER_VAULT_SEED, order_key.as_ref(), &[vault_bump]]];
  
  if refundable_amount > 0 {
    system_program::transfer(
      CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.order_vault.to_account_info(),
          to: ctx.accounts.payer.to_account_info(),
        },
        signer_seeds,
      ),
      refundable_amount,
    )?;
  }

  distribute(
    &ctx.accounts.system_program,
    ctx.accounts.order_vault.to_account_info(),
    signer_seeds,
    ctx.accounts.api_holder_vault.to_account_info(),
    ctx.accounts.treasury.to_account_info(),
    &ctx.accounts.order.split_recipients,
    ctx.remaining_accounts,
    non_refundable_amount,
    ctx.accounts.api_holder.fee_percentage,
  )?;
  
  ctx.accounts.order_payment.settled = true;
  
  emit_cpi!(OrderRefunded {
        order: order_key,
        order_payment: ctx.accounts.order_payment.key(),
        payer: ctx.accounts.payer.key(),
        refunded_amount: refundable_amount,
        settled_amount: non_refundable_amount,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct RefundOrder<'info> {
  pub auth: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  #[account(mut, seeds = [TREASURY_SEED], bump)]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), order.id.as_bytes()],
        bump = order.bump,
  )]
  pub order: Account<'info, PaymentOrder>,
  
  #[account(
        mut,
        seeds = [ORDER_PAYMENT_SEED, order.key().as_ref(), order_payment.payer.as_ref()],
        bump = order_payment.bump,
  )]
  pub order_payment: Account<'info, OrderPayment>,
  
  /// Refund destination — must be the payer recorded on `order_payment`.
  #[account(mut, address = order_payment.payer)]
  pub payer: SystemAccount<'info>,
  
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump = order.vault_bump,
  )]
  pub order_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}