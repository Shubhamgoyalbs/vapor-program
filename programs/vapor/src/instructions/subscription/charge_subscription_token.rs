use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::{ChargeFailureRecorded, SubscriptionCharged, SubscriptionLapsed};
use crate::send_token;
use crate::state::seeds::{SUBSCRIPTION_SEED, TREASURY_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{ApiHolder, Subscription, SubscriptionPolicy, SubscriptionStatus};

/// Unlike the old SPL-delegate design, there's no separate
/// report_failed_charge instruction needed here: because the funds sit
/// in an escrow account we own, we can read `vault.amount` and know
/// upfront whether a charge will succeed, BEFORE issuing any CPI. That's
/// what a failed-CPI-reverts-everything constraint actually rules out —
/// attempting a transfer we don't know will succeed. Checking the
/// balance first isn't a CPI, so branching on it and committing either
/// outcome (success, or a recorded retry / lapse) is safe in one
/// transaction.
pub fn handler(ctx: Context<ChargeSubscriptionToken>) -> Result<()> {
  ctx.accounts.api_holder
    .require_auth(
      ctx.accounts.auth.key
    )?;
  
  require!(
        !ctx.accounts.policy.is_native_sol(),
        ErrorCode::SubscriptionMintMismatch
    );
  require!(
        ctx.accounts.subscription.status == SubscriptionStatus::Active,
        ErrorCode::SubscriptionNotActive
    );
  let now = Clock::get()?.unix_timestamp;
  require!(
        now >= ctx.accounts.subscription.next_charge_at,
        ErrorCode::ChargeNotDue
    );
  
  let vault_balance = ctx.accounts.subscription_vault.amount;
  let amount_per_charge = ctx.accounts.policy.amount_per_charge;
  let subscription_account_info = ctx.accounts.subscription.to_account_info();
  
  let policy_key = ctx.accounts.policy.key();
  let subscriber_key = ctx.accounts.subscription.subscriber;
  let sub_bump = ctx.accounts.subscription.bump;
  let signer_seeds: &[&[&[u8]]] = &[&[
    SUBSCRIPTION_SEED,
    policy_key.as_ref(),
    subscriber_key.as_ref(),
    &[sub_bump],
  ]];
  
  if vault_balance >= amount_per_charge {
    send_token(
      &ctx.accounts.token_program,
      &ctx.accounts.mint,
      &ctx.accounts.subscription_vault,
      subscription_account_info.clone(),
      signer_seeds,
      &ctx.accounts.api_holder_vault,
      &ctx.accounts.treasury_token,
      &ctx.accounts.recipient_token_account,
      amount_per_charge,
      ctx.accounts.api_holder.fee_percentage,
    )?;
    
    let charge_interval = ctx.accounts.policy.charge_interval;
    let max_charges = ctx.accounts.policy.max_charges;
    let subscription = &mut ctx.accounts.subscription;
    subscription.charges_made = subscription.charges_made.saturating_add(1);
    subscription.retries_used = 0;
    subscription.next_charge_at = subscription.next_charge_at.saturating_add(charge_interval);
    
    let mut completed_refund = None;
    if max_charges > 0 && subscription.charges_made >= max_charges {
      subscription.status = SubscriptionStatus::Completed;
      let leftover = vault_balance.saturating_sub(amount_per_charge);
      if leftover > 0 {
        transfer_checked(
          CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
              from: ctx.accounts.subscription_vault.to_account_info(),
              mint: ctx.accounts.mint.to_account_info(),
              to: ctx.accounts.subscriber_token_account.to_account_info(),
              authority: subscription_account_info.clone(),
            },
            signer_seeds,
          ),
          leftover,
          ctx.accounts.mint.decimals,
        )?;
      }
      completed_refund = Some(leftover);
    }
    
    emit!(SubscriptionCharged {
            subscription: ctx.accounts.subscription.key(),
            policy: policy_key,
            amount: amount_per_charge,
            charges_made: ctx.accounts.subscription.charges_made,
            next_charge_at: ctx.accounts.subscription.next_charge_at,
            completed_refund,
        });
  } else {
    let retry_count = ctx.accounts.policy.retry_count;
    let subscription = &mut ctx.accounts.subscription;
    subscription.retries_used = subscription.retries_used.saturating_add(1);
    
    if subscription.retries_used >= retry_count {
      subscription.status = SubscriptionStatus::Lapsed;
      if vault_balance > 0 {
        transfer_checked(
          CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
              from: ctx.accounts.subscription_vault.to_account_info(),
              mint: ctx.accounts.mint.to_account_info(),
              to: ctx.accounts.subscriber_token_account.to_account_info(),
              authority: subscription_account_info,
            },
            signer_seeds,
          ),
          vault_balance,
          ctx.accounts.mint.decimals,
        )?;
      }
      emit!(SubscriptionLapsed {
                subscription: ctx.accounts.subscription.key(),
                refunded_amount: vault_balance,
            });
    } else {
      emit!(ChargeFailureRecorded {
                subscription: ctx.accounts.subscription.key(),
                retries_used: ctx.accounts.subscription.retries_used,
            });
    }
  }
  
  Ok(())
}

#[derive(Accounts)]
pub struct ChargeSubscriptionToken<'info> {
  #[account(mut)]
  pub auth: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  #[account(
        seeds = [SubscriptionPolicy::SEED_PREFIX, api_holder.key().as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, SubscriptionPolicy>,
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_SEED, policy.key().as_ref(), subscription.subscriber.as_ref()],
        bump = subscription.bump,
  )]
  pub subscription: Account<'info, Subscription>,
  
  #[account(address = policy.mint @ ErrorCode::SubscriptionMintMismatch)]
  pub mint: InterfaceAccount<'info, Mint>,
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump = subscription.vault_bump,
  )]
  pub subscription_vault: InterfaceAccount<'info, TokenAccount>,

  /// Refund destination for a lapse or a completion with leftover —
  /// not created speculatively via init_if_needed beyond what's
  /// needed, since the subscriber already has one (they funded the
  /// vault from it).
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscription.subscriber,
        associated_token::token_program = token_program,
  )]
  pub subscriber_token_account: InterfaceAccount<'info, TokenAccount>,

  #[account(address = policy.recipient)]
  pub recipient: SystemAccount<'info>,

  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program,
  )]
  pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_vault: InterfaceAccount<'info, TokenAccount>,
  
  #[account(
    mut,
    seeds = [TREASURY_SEED],
    bump,
  )]
  pub treasury: SystemAccount<'info>,
  
  #[account(
        init_if_needed,
        payer = auth,
        associated_token::mint = mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
  )]
  pub treasury_token: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}