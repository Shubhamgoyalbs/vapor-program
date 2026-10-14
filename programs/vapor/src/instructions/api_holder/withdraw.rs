use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::MIN_VAULT_LAMPORTS;
use crate::error::ErrorCode;
use crate::events::Withdrawn;
use crate::state::seeds::API_HOLDER_VAULT_SEED;
use crate::state::ApiHolder;

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_owner(&ctx.accounts.owner.key())?;
  
  let remaining = ctx
    .accounts
    .api_holder_vault
    .lamports()
    .checked_sub(amount)
    .ok_or(ErrorCode::BelowMinBalance)?;
  require!(remaining >= MIN_VAULT_LAMPORTS, ErrorCode::BelowMinBalance);
  
  let api_holder_key = ctx.accounts.api_holder.key();
  let vault_bump = ctx.accounts.api_holder.vault_bump;
  let signer_seeds: &[&[&[u8]]] = &[&[
    API_HOLDER_VAULT_SEED,
    api_holder_key.as_ref(),
    &[vault_bump],
  ]];
  
  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.api_holder_vault.to_account_info(),
        to: ctx.accounts.owner.to_account_info(),
      },
      signer_seeds,
    ),
    amount,
  )?;
  
  emit_cpi!(Withdrawn {
        api_holder: api_holder_key,
        amount,
        new_vault_balance: remaining,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct Withdraw<'info> {
  #[account(mut)]
  pub owner: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  /// CHECK: address fixed by seeds/bump against the value stored on
  /// `api_holder`; never deserialized, only debited lamports, and only
  /// ever as this PDA signing its own transfer via `signer_seeds`.
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}