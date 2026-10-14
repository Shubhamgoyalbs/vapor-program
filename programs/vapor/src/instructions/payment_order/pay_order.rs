use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::PERCENTAGE_PRECISION;
use crate::error::ErrorCode;
use crate::events::OrderPaid;
use crate::helper::distribute_from_signer;
use crate::state::seeds::{
  API_HOLDER_VAULT_SEED, ORDER_PAYMENT_SEED, PAYMENT_ORDER_VAULT_SEED, TREASURY_SEED,
};
use crate::state::{ApiHolder, OrderPayment, PaymentOrder};

/// Two paths, picked by `order.refundable_percentage`:
///   - 0: nothing will ever be refunded, so there's nothing to hold —
///     the payer's funds are split straight out to treasury / api fee /
///     recipients right here, and the payment is recorded already
///     `settled`. The payer is passed to `distribute_sol` as the
///     "vault" with empty signer_seeds, since a real signer needs no
///     PDA authorization.
///   - >0: the WHOLE amount goes into the order's pooled vault and
///     waits — for `settle_order` once the payer's window closes, or
///     `refund_order` if the maker refunds them first.
pub fn handler<'info>(ctx: Context<'info, PayOrder<'info>>) -> Result<()> {
  let now = Clock::get()?.unix_timestamp;
  let order = &mut ctx.accounts.order;
  
  require!(order.is_native_sol(), ErrorCode::OrderMintMismatch);
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
  let split_recipients = order.split_recipients.clone();
  
  let (refundable_amount, refund_valid_until, settled) = if refundable {
    system_program::transfer(
      CpiContext::new(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.payer.to_account_info(),
          to: ctx.accounts.order_vault.to_account_info(),
        },
      ),
      total_amount,
    )?;
    let refundable_amount = (total_amount as u128 * order.refundable_percentage as u128
      / PERCENTAGE_PRECISION as u128) as u64;
    let valid_until = now.saturating_add(order.refund_window_seconds.unwrap_or(0));
    (refundable_amount, Some(valid_until), false)
  } else {
    distribute_from_signer(
      &ctx.accounts.system_program.key(),
      &ctx.accounts.payer.to_account_info(),
      &ctx.accounts.api_holder_vault.to_account_info(),
      &ctx.accounts.treasury.to_account_info(),
      &split_recipients,
      ctx.remaining_accounts,
      ctx.accounts.api_holder.fee_percentage as u64,
      ctx.accounts.api_holder.fee_percentage,
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
pub struct PayOrder<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  /// api_holder's own SOL vault — receives the api fee cut.
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  /// Program-wide treasury PDA — receives PROGRAM_FEE_PERCENTAGE's cut.
  #[account(mut, seeds = [TREASURY_SEED], bump)]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        mut,
        seeds = [PaymentOrder::SEED_PREFIX, api_holder.key().as_ref(), order.id.as_bytes()],
        bump = order.bump,
  )]
  pub order: Account<'info, PaymentOrder>,
  
  /// One per (order, payer) — `init` is what stops double-paying.
  #[account(
        init,
        payer = payer,
        space = 8 + OrderPayment::INIT_SPACE,
        seeds = [ORDER_PAYMENT_SEED, order.key().as_ref(), payer.key().as_ref()],
        bump,
  )]
  pub order_payment: Account<'info, OrderPayment>,
  
  /// The order's pooled escrow. Only touched on the refundable path.
  #[account(
        mut,
        seeds = [PAYMENT_ORDER_VAULT_SEED, order.key().as_ref()],
        bump = order.vault_bump,
  )]
  pub order_vault: SystemAccount<'info>,
  
  /// Only used when `order.split_recipients` is all-`None`. Must be
  /// writable — it receives lamports.
  #[account(mut, address = api_holder.owner)]
  pub default_recipient: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}