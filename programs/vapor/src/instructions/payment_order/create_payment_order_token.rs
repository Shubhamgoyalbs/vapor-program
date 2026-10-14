use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::{
  MAX_ID_LEN, MAX_PAYER_ALLOWLIST, MAX_SPLIT_RECIPIENTS, MIN_VAULT_LAMPORTS,
  PERCENTAGE_PRECISION,
};
use crate::error::ErrorCode;
use crate::events::PaymentOrderCreated;
use crate::state::seeds::{API_HOLDER_VAULT_SEED, PAYMENT_ORDER_VAULT_SEED};
use crate::state::{ApiHolder, PaymentOrder, SplitRecipient};

/// Token-rail counterpart of `create_payment_order`. The pooled vault is
/// a real SPL/Token-2022 token account here, so it has to be `init`'d
/// against a real `mint` account at creation — which is why creation
/// itself splits by rail, not just payment.
pub fn handler(
  ctx: Context<CreatePaymentOrderToken>,
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
  
  // `auth` fronted rent for `order` AND the vault token account (read
  // post-init, so Token-2022 extension sizing is already accounted
  // for). Reimburse from the api_holder's own vault.
  let rent_reimbursement = ctx
    .accounts
    .order
    .to_account_info()
    .lamports()
    .checked_add(ctx.accounts.order_vault.to_account_info().lamports())
    .ok_or(ErrorCode::BelowMinBalance)?;
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
  order.mint = ctx.accounts.mint.key();
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
        vault: ctx.accounts.order_vault.key()
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreatePaymentOrderToken<'info> {
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
  
  /// The mint every payment on this order is denominated in.
  /// Token-2022 compatible.
  pub mint: Box<InterfaceAccount<'info, Mint>>,
  
  #[account(
        init,
        payer = auth,
        space = 8 + PaymentOrder::INIT_SPACE,
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), id.as_bytes()],
        bump,
  )]
  pub order: Box<Account<'info, PaymentOrder>>,
  
  /// Pooled escrow for every payer's pending refundable slice.
  /// Authority is the order PDA itself — its own seeds sign the
  /// transfers out (settle_order_token / refund_order_token).
  #[account(
        init,
        payer = auth,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = order,
        token::token_program = token_program,
  )]
  pub order_vault: Box<InterfaceAccount<'info, TokenAccount>>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub system_program: Program<'info, System>,
}