use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::OrderRefunded;
use crate::helper::distribute_token;
use crate::state::seeds::{ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Token-rail counterpart of `refund_order` — maker-initiated, one
/// payer at a time, from the pooled vault.
pub fn handler<'info>(ctx: Context<'info, RefundOrderToken<'info>>) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  let order = &ctx.accounts.order;
  let order_payment = &ctx.accounts.order_payment;
  
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
  let api_holder_key = order.api_holder;
  let order_id = order.id.clone();
  let order_bump = order.bump;
  // The pooled vault's SPL authority is the ORDER PDA — its own seeds
  // sign, not the vault's.
  let signer_seeds: &[&[&[u8]]] = &[&[
    PaymentOrder::SEED_PREFIX,
    api_holder_key.as_ref(),
    order_id.as_bytes(),
    &[order_bump],
  ]];
  
  if refundable_amount > 0 {
    transfer_checked(
      CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        TransferChecked {
          from: ctx.accounts.order_vault.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          to: ctx.accounts.payer_token_account.to_account_info(),
          authority: ctx.accounts.order.to_account_info(),
        },
        signer_seeds,
      ),
      refundable_amount,
      ctx.accounts.mint.decimals,
    )?;
  }

  distribute_token(
    &ctx.accounts.token_program,
    &ctx.accounts.mint,
    &ctx.accounts.order_vault.to_account_info(),
    &ctx.accounts.order.to_account_info(),
    ctx.accounts.order.split_recipients.clone(),
    &ctx.accounts.api_holder_vault.to_account_info(),
    &ctx.accounts.treasury_token_account.to_account_info(),
    ctx.remaining_accounts,
    non_refundable_amount,
    ctx.accounts.api_holder.fee_percentage,
  )?;
  
  ctx.accounts.order_payment.settled = true;
  
  emit_cpi!(OrderRefunded {
        order: order_key,
        order_payment: ctx.accounts.order_payment.key(),
        payer: ctx.accounts.order_payment.payer,
        refunded_amount: refundable_amount,
        settled_amount: non_refundable_amount,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct RefundOrderToken<'info> {
  #[account(mut)]
  pub auth: Signer<'info>,
  
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

  /// The recorded payer's wallet — a real account (pinned by `address`)
  /// because `init_if_needed` needs an account, not a bare Pubkey, as
  /// the ATA authority.
  #[account(address = order_payment.payer)]
  pub payer: SystemAccount<'info>,

  /// Refund destination — the recorded payer's ATA. Created on demand
  /// (maker pays) in case the payer closed it since paying.
  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
  )]
  pub payer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_vault: Box<InterfaceAccount<'info, TokenAccount>>,
  
  #[account(seeds = [TREASURY_SEED], bump)]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
  )]
  pub treasury_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
  
  /// api_holder's owner wallet — same reason as `payer` above.
  #[account(address = api_holder.owner)]
  pub owner: SystemAccount<'info>,
  
  /// Only used when `order.split_recipients` is all-`None`.
  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
  )]
  pub default_recipient: Box<InterfaceAccount<'info, TokenAccount>>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}