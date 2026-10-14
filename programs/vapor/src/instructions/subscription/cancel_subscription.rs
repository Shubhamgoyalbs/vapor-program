use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::SubscriptionCancelled;
use crate::state::seeds::{SUBSCRIPTION_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{Subscription, SubscriptionStatus};

pub fn handler(ctx: Context<CancelSubscription>) -> Result<()> {
  require!(
        ctx.accounts.subscription.status == SubscriptionStatus::Active,
        ErrorCode::SubscriptionNotActive
    );
  
  let refund_amount = ctx.accounts.subscription_vault.lamports();
  if refund_amount > 0 {
    // `subscription_vault` is its own separately-seeded PDA (SUBSCRIPTION_VAULT_SEED),
    // not subscription's — same point as charge_subscription_sol.rs.
    let subscription_key = ctx.accounts.subscription.key();
    let vault_bump = ctx.accounts.subscription.vault_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
      SUBSCRIPTION_VAULT_SEED,
      subscription_key.as_ref(),
      &[vault_bump],
    ]];

    system_program::transfer(
      CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.subscription_vault.to_account_info(),
          to: ctx.accounts.subscriber.to_account_info(),
        },
        signer_seeds,
      ),
      refund_amount,
    )?;
  }
  
  ctx.accounts.subscription.status = SubscriptionStatus::Cancelled;
  
  emit!(SubscriptionCancelled {
        subscription: ctx.accounts.subscription.key(),
        refunded_amount: refund_amount,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct CancelSubscription<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_SEED, subscription.policy.as_ref(), subscriber.key().as_ref()],
        bump = subscription.bump,
  )]
  pub subscription: Account<'info, Subscription>,
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump = subscription.vault_bump,
  )]
  pub subscription_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}