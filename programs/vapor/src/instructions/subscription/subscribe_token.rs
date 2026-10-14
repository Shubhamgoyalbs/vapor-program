use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::Subscribed;
use crate::state::seeds::{SUBSCRIPTION_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{Subscription, SubscriptionPolicy, SubscriptionStatus};

pub fn handler(ctx: Context<SubscribeToken>, initial_deposit: u64) -> Result<()> {
  if initial_deposit > 0 {
    transfer_checked(
      CpiContext::new(
        ctx.accounts.token_program.key(),
        TransferChecked {
          from: ctx.accounts.subscriber_token_account.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          to: ctx.accounts.subscription_vault.to_account_info(),
          authority: ctx.accounts.subscriber.to_account_info(),
        },
      ),
      initial_deposit,
      ctx.accounts.mint.decimals,
    )?;
  }

  let subscription = &mut ctx.accounts.subscription;
  subscription.policy = ctx.accounts.policy.key();
  subscription.subscriber = ctx.accounts.subscriber.key();
  subscription.vault = ctx.accounts.subscription_vault.key();
  subscription.vault_bump = ctx.bumps.subscription_vault;
  subscription.charges_made = 0;
  subscription.retries_used = 0;
  // Due immediately — the scheduler is expected to call
  // charge_subscription right after this to collect the first charge,
  // through the same code path as every later one.
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
pub struct SubscribeToken<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  #[account(address = policy.mint @ ErrorCode::SubscriptionMintMismatch)]
  pub mint: InterfaceAccount<'info, Mint>,
  
  pub policy: Account<'info, SubscriptionPolicy>,
  
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscriber,
        associated_token::token_program = token_program,
  )]
  pub subscriber_token_account: InterfaceAccount<'info, TokenAccount>,
  
  /// One per (policy, subscriber) — `init` here is what stops
  /// double-subscribing to the same policy.
  #[account(
        init,
        payer = subscriber,
        space = 8 + Subscription::INIT_SPACE,
        seeds = [SUBSCRIPTION_SEED, policy.key().as_ref(), subscriber.key().as_ref()],
        bump,
  )]
  pub subscription: Account<'info, Subscription>,
  
  /// This subscription's own escrow — created unconditionally, same
  /// reasoning as everywhere else in this program. Authority is the
  /// Subscription PDA itself.
  #[account(
        init,
        payer = subscriber,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = subscription,
        token::token_program = token_program,
  )]
  pub subscription_vault: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}