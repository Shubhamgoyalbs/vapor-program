use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::SubscriptionToppedUp;
use crate::state::seeds::SUBSCRIPTION_VAULT_SEED;
use crate::state::{Subscription, SubscriptionStatus};

pub fn handler(ctx: Context<TopUpSubscription>, amount: u64) -> Result<()> {
  require!(
        ctx.accounts.subscription.status == SubscriptionStatus::Active,
        ErrorCode::SubscriptionNotActive
    );
  
  if amount > 0 {
    system_program::transfer(
      CpiContext::new(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.subscriber.to_account_info(),
          to: ctx.accounts.subscription_vault.to_account_info(),
        },
      ),
      amount,
    )?;
  }

  emit!(SubscriptionToppedUp {
        subscription: ctx.accounts.subscription.key(),
        amount,
        new_vault_balance: ctx.accounts.subscription_vault.lamports() + amount,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct TopUpSubscription<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  #[account(
        seeds = [crate::state::seeds::SUBSCRIPTION_SEED, subscription.policy.as_ref(), subscriber.key().as_ref()],
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