use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
  transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::ErrorCode;
use crate::events::SubscriptionToppedUp;
use crate::state::seeds::SUBSCRIPTION_VAULT_SEED;
use crate::state::{Subscription, SubscriptionStatus};

pub fn handler(ctx: Context<TopUpSubscriptionToken>, amount: u64) -> Result<()> {
  require!(
        ctx.accounts.subscription.status == SubscriptionStatus::Active,
        ErrorCode::SubscriptionNotActive
    );
  
  if amount > 0 {
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
      amount,
      ctx.accounts.mint.decimals,
    )?;
  }

  emit!(SubscriptionToppedUp {
        subscription: ctx.accounts.subscription.key(),
        amount,
        new_vault_balance: ctx.accounts.subscription_vault.amount + amount,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct TopUpSubscriptionToken<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  pub mint: InterfaceAccount<'info, Mint>,
  
  // Seeds use the live signer's own key, not subscription.subscriber.
  #[account(
        seeds = [crate::state::seeds::SUBSCRIPTION_SEED, subscription.policy.as_ref(), subscriber.key().as_ref()],
        bump = subscription.bump,
  )]
  pub subscription: Account<'info, Subscription>,
  
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscriber,
        associated_token::token_program = token_program,
  )]
  pub subscriber_token_account: InterfaceAccount<'info, TokenAccount>,
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump = subscription.vault_bump,
  )]
  pub subscription_vault: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub system_program: Program<'info, System>,
}