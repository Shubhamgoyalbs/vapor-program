use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::OrderSettled;
use crate::helper::distribute;
use crate::state::seeds::{
  API_HOLDER_VAULT_SEED, ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED,
};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Permissionless — your off-chain worker calls this once a payer's
/// refund window has closed. Sweeps THAT payer's full contribution
/// (`order.total_amount`, not the vault's whole balance — the vault is
/// pooled, other payers' funds are still sitting in it) out to
/// treasury / api fee / recipients.
pub fn handler<'info>(ctx: Context<'info, SettleOrder<'info>>) -> Result<()> {
  require!(
        ctx.accounts.order.is_native_sol(),
        ErrorCode::OrderMintMismatch
    );
  require!(!ctx.accounts.order_payment.settled, ErrorCode::OrderAlreadySettled);
  
  // A payment with no window is always settled at pay-time, so
  // reaching here with None means something's off — treat as settled.
  let deadline = ctx
    .accounts
    .order_payment
    .refund_valid_until
    .ok_or(ErrorCode::OrderAlreadySettled)?;
  require!(
        Clock::get()?.unix_timestamp > deadline,
        ErrorCode::RefundWindowStillOpen
    );
  
  let amount = ctx.accounts.order.total_amount;
  let order_key = ctx.accounts.order.key();
  let vault_bump = ctx.accounts.order.vault_bump;
  // SOL rail: the vault is a plain lamport PDA with no SPL "authority"
  // — it signs for itself with its own seeds.
  let signer_seeds: &[&[&[u8]]] =
    &[&[PAYMENT_ORDER_VAULT_SEED, order_key.as_ref(), &[vault_bump]]];
  
  distribute(
    &ctx.accounts.system_program,
    ctx.accounts.order_vault.to_account_info(),
    signer_seeds,
    ctx.accounts.api_holder_vault.to_account_info(),
    ctx.accounts.treasury.to_account_info(),
    &ctx.accounts.order.split_recipients,
    ctx.remaining_accounts,
    amount,
    ctx.accounts.api_holder.fee_percentage,
  )?;
  
  ctx.accounts.order_payment.settled = true;
  
  emit_cpi!(OrderSettled {
        order: order_key,
        order_payment: ctx.accounts.order_payment.key(),
        amount,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct SettleOrder<'info> {
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
  
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump = order.vault_bump,
  )]
  pub order_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}