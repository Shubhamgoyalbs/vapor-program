use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
  transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::ErrorCode;
use crate::events::SubscriptionCancelled;
use crate::state::seeds::{SUBSCRIPTION_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{Subscription, SubscriptionStatus};

pub fn handler(ctx: Context<CancelSubscriptionToken>) -> Result<()> {
  require!(
        ctx.accounts.subscription.status == SubscriptionStatus::Active,
        ErrorCode::SubscriptionNotActive
    );
  
  let refund_amount = ctx.accounts.subscription_vault.amount;
  if refund_amount > 0 {
    let policy_key = ctx.accounts.subscription.policy;
    let subscriber_key = ctx.accounts.subscription.subscriber;
    let bump = ctx.accounts.subscription.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
      SUBSCRIPTION_SEED,
      policy_key.as_ref(),
      subscriber_key.as_ref(),
      &[bump],
    ]];

    transfer_checked(
      CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        TransferChecked {
          from: ctx.accounts.subscription_vault.to_account_info(),
          mint: ctx.accounts.mint.to_account_info(),
          to: ctx.accounts.subscriber_token_account.to_account_info(),
          authority: ctx.accounts.subscription.to_account_info(),
        },
        signer_seeds,
      ),
      refund_amount,
      ctx.accounts.mint.decimals,
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
pub struct CancelSubscriptionToken<'info> {
  #[account(mut)]
  pub subscriber: Signer<'info>,
  
  pub mint: InterfaceAccount<'info, Mint>,
  
  // Seeds use the live signer's own key, not subscription.subscriber.
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
  pub subscription_vault: InterfaceAccount<'info, TokenAccount>,
  
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscriber,
        associated_token::token_program = token_program,
  )]
  pub subscriber_token_account: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
}