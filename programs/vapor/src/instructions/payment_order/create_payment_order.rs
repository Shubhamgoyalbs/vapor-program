use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::{
  MAX_ID_LEN, MAX_PAYER_ALLOWLIST, MAX_SPLIT_RECIPIENTS, MIN_VAULT_LAMPORTS,
  NATIVE_SOL_MINT_SENTINEL, PERCENTAGE_PRECISION,
};
use crate::error::ErrorCode;
use crate::events::PaymentOrderCreated;
use crate::state::seeds::{API_HOLDER_VAULT_SEED, PAYMENT_ORDER_VAULT_SEED};
use crate::state::{ApiHolder, PaymentOrder, SplitRecipient};

pub fn handler(
  ctx: Context<CreatePaymentOrder>,
  id: String,
  total_amount: u64,
  split_recipients: Vec<SplitRecipient>,
  allowed_payers: Vec<Pubkey>,
  mut payer_cap: u16,
  expiry: Option<i64>,
  refundable_percentage: u32,
  refund_window_seconds: Option<i64>,
) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  require!(id.len() <= MAX_ID_LEN, ErrorCode::IdTooLong);
  
  let percentage_sum: u32 = split_recipients
    .iter()
    .map(|s| s.percentage)
    .sum();
  require!(
        percentage_sum == 0 || percentage_sum == PERCENTAGE_PRECISION,
        ErrorCode::SplitPercentageMismatch
    );
  
  let allowlist_len = allowed_payers.iter().count() as u16;
  if allowlist_len > 0 && (payer_cap == 0 || payer_cap > allowlist_len) {
    payer_cap = allowlist_len;
  }
  
  require!(
        refundable_percentage <= PERCENTAGE_PRECISION,
        ErrorCode::InvalidRefundablePercentage
    );
  let refund_terms_ok = match (refundable_percentage, refund_window_seconds) {
    (0, None) => true,
    (p, Some(secs)) if p > 0 => secs > 0,
    _ => false,
  };
  require!(refund_terms_ok, ErrorCode::InvalidRefundWindow);
  
  // `auth` fronted the rent for `order` (it had to — it's the tx
  // signer). Reimburse it from the api_holder's own vault.
  let rent_reimbursement = ctx.accounts.order.to_account_info().lamports();
  let vault_remaining = ctx
    .accounts
    .api_holder_vault
    .lamports()
    .checked_sub(rent_reimbursement)
    .ok_or(ErrorCode::BelowMinBalance)?;
  require!(vault_remaining >= MIN_VAULT_LAMPORTS, ErrorCode::BelowMinBalance);
  
  let api_holder_key = ctx.accounts.api_holder.key();
  let vault_bump = ctx.accounts.api_holder.vault_bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[API_HOLDER_VAULT_SEED, api_holder_key.as_ref(), &[vault_bump]]];
  
  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.api_holder_vault.to_account_info(),
        to: ctx.accounts.auth.to_account_info(),
      },
      signer_seeds,
    ),
    rent_reimbursement,
  )?;
  
  let now = Clock::get()?.unix_timestamp;
  let order = &mut ctx.accounts.order;
  order.id = id;
  order.api_holder = api_holder_key;
  order.mint = NATIVE_SOL_MINT_SENTINEL;
  order.total_amount = total_amount;
  order.split_recipients = split_recipients;
  order.allowed_payers = allowed_payers;
  order.payer_cap = payer_cap;
  order.payers_paid = 0;
  order.is_closed = false;
  order.expiry = expiry;
  order.refundable_percentage = refundable_percentage;
  order.refund_window_seconds = refund_window_seconds;
  order.vault_bump = ctx.bumps.order_vault;
  order.created_at = now;
  order.bump = ctx.bumps.order;
  order._reserved = [0u8; 16];
  
  emit_cpi!(PaymentOrderCreated {
        order: order.key(),
        api_holder: order.api_holder,
        mint: order.mint,
        total_amount,
        payer_cap,
        expiry,
        refundable_percentage,
        refund_window_seconds,
        vault: ctx.accounts.order_vault.key(),
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreatePaymentOrder<'info> {
  #[account(mut)]
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
  
  #[account(
        init,
        payer = auth,
        space = 8 + PaymentOrder::INIT_SPACE,
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), id.as_bytes()],
        bump,
  )]
  pub order: Account<'info, PaymentOrder>,
  
  /// This order's pooled escrow — not `init`'d, starts empty; the
  /// first payer's `pay_order` call is what actually funds it, same
  /// "transfer creates it" pattern as every other plain lamport PDA
  /// in this program.
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump,
  )]
  pub order_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}