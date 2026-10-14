use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::Subscribed;
use crate::state::seeds::{SUBSCRIPTION_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{Subscription, SubscriptionPolicy, SubscriptionStatus};

pub fn handler(ctx: Context<Subscribe>, initial_deposit: u64) -> Result<()> {
  require!(
        ctx.accounts.policy.is_native_sol(),
        ErrorCode::SubscriptionMintMismatch
    );
  
  if initial_deposit > 0 {
    system_program::transfer(
      CpiContext::new(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.subscriber.to_account_info(),
          to: ctx.accounts.subscription_vault.to_account_info(),
        },
      ),
      initial_deposit,
    )?;
  }
  
  let subscription = &mut ctx.accounts.subscription;
  subscription.policy = ctx.accounts.policy.key();
  subscription.subscriber = ctx.accounts.subscriber.key();
  subscription.vault = ctx.accounts.subscription_vault.key();
  subscription.vault_bump = ctx.bumps.subscription_vault;
  subscription.charges_made = 0;
  subscription.retries_used = 0;
  subscription.next_charge_at = Clock::get()?.unix_timestamp;
  subscription.status = SubscriptionStatus::Active;
  subscription.bump = ctx.bumps.subscription;
  
  emit!(Subscribed {
        subscription: subscription.key(),
        policy: subscription.policy,
        subscriber: subscription.subscriber,
        initial_deposit,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct Subscribe<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  pub policy: Account<'info, SubscriptionPolicy>,
  
  #[account(
        init,
        payer = subscriber,
        space = 8 + Subscription::INIT_SPACE,
        seeds = [SUBSCRIPTION_SEED, policy.key().as_ref(), subscriber.key().as_ref()],
        bump,
  )]
  pub subscription: Account<'info, Subscription>,
  
  /// System-owned PDA holding native SOL — the vault's own seeds will
  /// sign its outgoing transfers later (charge / cancel), same
  /// pattern as every other SOL-rail vault in this program.
  #[account(
        mut,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump,
  )]
  pub subscription_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}