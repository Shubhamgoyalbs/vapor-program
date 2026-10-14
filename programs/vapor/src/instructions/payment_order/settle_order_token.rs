use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::error::ErrorCode;
use crate::events::OrderSettled;
use crate::helper::distribute_token;
use crate::state::seeds::{ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Token-rail counterpart of `settle_order`. Permissionless; sweeps one
/// payer's `order.total_amount` out of the pooled vault once their
/// window's closed.
pub fn handler<'info>(ctx: Context<'info, SettleOrderToken<'info>>) -> Result<()> {
  require!(!ctx.accounts.order_payment.settled, ErrorCode::OrderAlreadySettled);
  
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
  let api_holder_key = ctx.accounts.order.api_holder;
  let order_id = ctx.accounts.order.id.clone();
  let order_bump = ctx.accounts.order.bump;
  // Token rail: the pooled vault's SPL `authority` is the ORDER PDA
  // (set at create_payment_order_token), so the order's own seeds sign.
  let _signer_seeds: &[&[&[u8]]] = &[&[
    PaymentOrder::SEED_PREFIX,
    api_holder_key.as_ref(),
    order_id.as_bytes(),
    &[order_bump],
  ]];
  
  distribute_token(
    &ctx.accounts.token_program,
    &ctx.accounts.mint,
    &ctx.accounts.order_vault.to_account_info(),
    &ctx.accounts.order.to_account_info(),
    ctx.accounts.order.split_recipients.clone(),
    &ctx.accounts.api_holder_vault.to_account_info(),
    &ctx.accounts.treasury_token_account.to_account_info(),
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
pub struct SettleOrderToken<'info> {
  /// Whoever cranks this — pays only for ATAs below if missing.
  #[account(mut)]
  pub caller: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Box<Account<'info, ApiHolder>>,
  
  #[account(
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), order.id.as_bytes()],
        bump = order.bump,
  )]
  pub order: Box<Account<'info, PaymentOrder>>,
  
  #[account(
        mut,
        seeds = [ORDER_PAYMENT_SEED, order.key().as_ref(), order_payment.payer.as_ref()],
        bump = order_payment.bump,
  )]
  pub order_payment: Box<Account<'info, OrderPayment>>,
  
  #[account(address = order.mint @ ErrorCode::OrderMintMismatch)]
  pub mint: Box<InterfaceAccount<'info, Mint>>,
  
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump = order.vault_bump,
  )]
  pub order_vault: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        init_if_needed,
        payer = caller,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_vault: Box<InterfaceAccount<'info, TokenAccount>>,
  
  #[account(seeds = [TREASURY_SEED], bump)]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        init_if_needed,
        payer = caller,
        associated_token::mint = mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
  )]
  pub treasury_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
  
  /// Plain wallet, passed as a real account (pinned by `address`) because
  /// `init_if_needed` needs an account — not a bare Pubkey — as the ATA
  /// authority.
  #[account(address = api_holder.owner)]
  pub owner: SystemAccount<'info>,
  
  /// Only used when `order.split_recipients` is all-`None`.
  #[account(
        init_if_needed,
        payer = caller,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
  )]
  pub default_recipient: Box<InterfaceAccount<'info, TokenAccount>>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}