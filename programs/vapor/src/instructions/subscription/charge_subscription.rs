use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::{ChargeFailureRecorded, SubscriptionCharged, SubscriptionLapsed};
use crate::send;
use crate::state::seeds::{API_HOLDER_VAULT_SEED, TREASURY_SEED, SUBSCRIPTION_SEED, SUBSCRIPTION_VAULT_SEED};
use crate::state::{ApiHolder, Subscription, SubscriptionPolicy, SubscriptionStatus};

pub fn handler(ctx: Context<ChargeSubscription>) -> Result<()> {
  ctx.accounts.api_holder
    .require_auth(
      ctx.accounts.auth.key
    )?;
  
  require!(
        ctx.accounts.policy.is_native_sol(),
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
  
  let vault_balance = ctx.accounts.subscription_vault.lamports();
  let amount_per_charge = ctx.accounts.policy.amount_per_charge;
  let policy_key = ctx.accounts.policy.key();
  
  // Unlike the token rail (where the vault's SPL `authority` is the
  // `subscription` PDA), there's no authority-delegation concept for a
  // plain lamport account — `vault` is its OWN, separately-seeded PDA
  // (SUBSCRIPTION_VAULT_SEED, not SUBSCRIPTION_SEED), and it signs for
  // itself.
  let subscription_key = ctx.accounts.subscription.key();
  let vault_bump = ctx.accounts.subscription.vault_bump;
  let signer_seeds: &[&[&[u8]]] = &[&[
    SUBSCRIPTION_VAULT_SEED,
    subscription_key.as_ref(),
    &[vault_bump],
  ]];
  
  if vault_balance >= amount_per_charge {
    send(
      &ctx.accounts.system_program,
      ctx.accounts.subscription_vault.to_account_info(),
      signer_seeds,
      ctx.accounts.api_holder_vault.to_account_info(),
      ctx.accounts.treasury.to_account_info(),
      ctx.accounts.recipient.to_account_info(),
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
        system_program::transfer(
          CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
              from: ctx.accounts.subscription_vault.to_account_info(),
              to: ctx.accounts.subscriber.to_account_info(),
            },
            signer_seeds,
          ),
          leftover,
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
        system_program::transfer(
          CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
              from: ctx.accounts.subscription_vault.to_account_info(),
              to: ctx.accounts.subscriber.to_account_info(),
            },
            signer_seeds,
          ),
          vault_balance,
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
pub struct ChargeSubscription<'info> {
  #[account(mut)]
  pub auth: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  #[account(
    mut,
    seeds = [TREASURY_SEED],
    bump,
  )]
  pub treasury: SystemAccount<'info>,
  
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
  
  #[account(
        mut,
        seeds = [SUBSCRIPTION_VAULT_SEED, subscription.key().as_ref()],
        bump = subscription.vault_bump,
  )]
  pub subscription_vault: SystemAccount<'info>,
  
  /// Lapse / completion-leftover refund destination — the
  /// subscriber's own wallet, no ATA needed for a lamport transfer.
  #[account(mut, address = subscription.subscriber)]
  pub subscriber: SystemAccount<'info>,
  
  #[account(address = policy.recipient)]
  pub recipient: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}