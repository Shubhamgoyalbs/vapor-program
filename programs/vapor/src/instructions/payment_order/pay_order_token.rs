use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::constants::PERCENTAGE_PRECISION;
use crate::error::ErrorCode;
use crate::events::OrderPaid;
use crate::helper::distribute_token_from_signer;
use crate::state::seeds::{ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Token-rail counterpart of `pay_order` — same two paths (see there).
/// On the non-refundable path the payer's own token account is passed to
/// `distribute` as the source, with empty signer_seeds.
pub fn handler<'info>(ctx: Context<'info, PayOrderToken<'info>>) -> Result<()> {
  let now = Clock::get()?.unix_timestamp;
  let order = &mut ctx.accounts.order;
  
  require!(!order.is_closed, ErrorCode::OrderClosed);
  if let Some(expiry) = order.expiry {
    require!(now <= expiry, ErrorCode::OrderExpired);
  }
  if order.payer_cap > 0 {
    require!(order.payers_paid < order.payer_cap, ErrorCode::PayerCapReached);
  }
  require!(
        order.payer_is_eligible(&ctx.accounts.payer.key()),
        ErrorCode::UnauthorizedPayer
    );
  
  order.payers_paid = order.payers_paid.saturating_add(1);
  let total_amount = order.total_amount;
  let refundable = order.refundable_percentage > 0;
  
  let (refundable_amount, refund_valid_until, settled) = if refundable {
    transfer_checked(
      CpiContext::new(
        ctx.accounts.token_program.key(),
        TransferChecked {
          from: ctx.accounts.payer_token_account.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          to: ctx.accounts.order_vault.to_account_info(),
          authority: ctx.accounts.payer.to_account_info(),
        },
      ),
      total_amount,
      ctx.accounts.mint.decimals,
    )?;
    let refundable_amount = (total_amount as u128 * order.refundable_percentage as u128
      / PERCENTAGE_PRECISION as u128) as u64;
    let valid_until = now.saturating_add(order.refund_window_seconds.unwrap_or(0));
    (refundable_amount, Some(valid_until), false)
  } else {
    distribute_token_from_signer(
      &ctx.accounts.token_program.key(),
      &ctx.accounts.mint.to_account_info(),
      &ctx.accounts.payer_token_account.to_account_info(),
      &ctx.accounts.payer.to_account_info(),
      &order.split_recipients,
      &ctx.accounts.api_holder_vault.to_account_info(),
      &ctx.accounts.treasury_token_account.to_account_info(),
      ctx.remaining_accounts,
      total_amount,
      ctx.accounts.api_holder.fee_percentage,
      ctx.accounts.mint.decimals,
    )?;
    (0, None, true)
  };
  
  let order_payment = &mut ctx.accounts.order_payment;
  order_payment.order = order.key();
  order_payment.payer = ctx.accounts.payer.key();
  order_payment.paid_at = now;
  order_payment.refundable_amount = refundable_amount;
  order_payment.refund_valid_until = refund_valid_until;
  order_payment.settled = settled;
  order_payment.bump = ctx.bumps.order_payment;
  
  emit_cpi!(OrderPaid {
        order: order.key(),
        order_payment: order_payment.key(),
        payer: order_payment.payer,
        amount: total_amount,
        refundable_amount,
        settled_immediately: settled,
        paid_at: now,
        refund_valid_until,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct PayOrderToken<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Box<Account<'info, ApiHolder>>,
  
  #[account(
        mut,
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), order.id.as_bytes()],
        bump = order.bump,
  )]
  pub order: Box<Account<'info, PaymentOrder>>,
  
  #[account(address = order.mint @ ErrorCode::OrderMintMismatch)]
  pub mint: Box<InterfaceAccount<'info, Mint>>,
  
  /// The payer's own existing token account — not created here.
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
  )]
  pub payer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
  
  /// One per (order, payer) — `init` is what stops double-paying.
  #[account(
        init,
        payer = payer,
        space = 8 + OrderPayment::INIT_SPACE,
        seeds = [ORDER_PAYMENT_SEED, order.key().as_ref(), payer.key().as_ref()],
        bump,
  )]
  pub order_payment: Box<Account<'info, OrderPayment>>,
  
  /// The order's pooled escrow. Only touched on the refundable path.
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump = order.vault_bump,
  )]
  pub order_vault: Box<InterfaceAccount<'info, TokenAccount>>,

  /// api_holder's per-mint fee ATA.
  #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_vault: Box<InterfaceAccount<'info, TokenAccount>>,
  
  /// Program-wide treasury PDA — only used as the ATA authority below.
  #[account(seeds = [TREASURY_SEED], bump)]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        init_if_needed,
        payer = payer,
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
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
  )]
  pub default_recipient: Box<InterfaceAccount<'info, TokenAccount>>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}