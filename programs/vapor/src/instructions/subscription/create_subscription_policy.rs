use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::{MAX_ID_LEN, MIN_VAULT_LAMPORTS, NATIVE_SOL_MINT_SENTINEL};
use crate::error::ErrorCode;
use crate::events::SubscriptionPolicyCreated;
use crate::state::seeds::API_HOLDER_VAULT_SEED;
use crate::state::{ApiHolder, SubscriptionPolicy};

pub fn handler(
  ctx: Context<CreateSubscriptionPolicy>,
  id: String,
  recipient: Pubkey,
  amount_per_charge: u64,
  charge_interval: i64,
  max_charges: u32,
  retry_count: u8,
) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  require!(id.len() <= MAX_ID_LEN, ErrorCode::IdTooLong);
  require!(charge_interval > 0, ErrorCode::InvalidChargeInterval);
  require!(retry_count <= 10, ErrorCode::InvalidRetryCount);
  
  let rent_reimbursement = ctx.accounts.policy.to_account_info().lamports();
  let vault_remaining = ctx
    .accounts
    .api_holder_vault
    .lamports()
    .checked_sub(rent_reimbursement)
    .ok_or(ErrorCode::BelowMinBalance)?;
  require!(vault_remaining >= MIN_VAULT_LAMPORTS, ErrorCode::BelowMinBalance);
  
  let api_holder_key = ctx.accounts.api_holder.key();
  let vault_bump = ctx.accounts.api_holder.vault_bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[API_HOLDER_VAULT_SEED, api_holder_key.as_ref(), &[vault_bump]]];
  
  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.api_holder_vault.to_account_info(),
        to: ctx.accounts.auth.to_account_info(),
      },
      signer_seeds,
    ),
    rent_reimbursement,
  )?;
  
  let policy = &mut ctx.accounts.policy;
  policy.id = id;
  policy.api_holder = api_holder_key;
  policy.mint = NATIVE_SOL_MINT_SENTINEL;
  policy.recipient = recipient;
  policy.amount_per_charge = amount_per_charge;
  policy.charge_interval = charge_interval;
  policy.max_charges = max_charges;
  policy.retry_count = retry_count;
  policy.created_at = Clock::get()?.unix_timestamp;
  policy.bump = ctx.bumps.policy;
  policy._reserved = [0u8; 16];
  
  emit!(SubscriptionPolicyCreated {
        policy: policy.key(),
        api_holder: policy.api_holder,
        mint: policy.mint,
        recipient,
        amount_per_charge,
        charge_interval,
        max_charges,
        retry_count,
    });
  
  Ok(())
}

#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreateSubscriptionPolicy<'info> {
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
        init,
        payer = auth,
        space = 8 + SubscriptionPolicy::INIT_SPACE,
        seeds = [SubscriptionPolicy::SEED_PREFIX, api_holder.key().as_ref(), id.as_bytes()],
        bump,
  )]
  pub policy: Account<'info, SubscriptionPolicy>,
  
  pub system_program: Program<'info, System>,
}