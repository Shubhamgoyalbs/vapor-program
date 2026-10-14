use anchor_lang::prelude::*;
use anchor_lang::{
  system_program
};
use crate::constants::{AUTH_GAS_LAMPORTS, PERCENTAGE_PRECISION, PROGRAM_FEE_PERCENTAGE, MIN_VAULT_LAMPORTS};
use crate::error::ErrorCode;
use crate::events::ApiHolderInitialized;
use crate::state::seeds::API_HOLDER_VAULT_SEED;
use crate::state::ApiHolder;

pub fn handler(ctx: Context<InitApiHolder>, fee_percentage: u32) -> Result<()> {
  require!(
        fee_percentage.saturating_add(PROGRAM_FEE_PERCENTAGE) <= PERCENTAGE_PRECISION,
        ErrorCode::FeePercentageTooHigh
    );
  
  // owner -> auth: Balance to maintain such the Privy wallet can pay its
  // own tx fees from the start.
  system_program::transfer(
    CpiContext::new(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.owner.to_account_info(),
        to: ctx.accounts.auth.to_account_info(),
      },
    ),
    AUTH_GAS_LAMPORTS,
  )?;
  
  // owner -> api_holder: Balance to maintain such the pda can pay for
  // further account creation
  system_program::transfer(
    CpiContext::new(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.owner.to_account_info(),
        to: ctx.accounts.api_holder_vault.to_account_info(),
      },
    ),
    MIN_VAULT_LAMPORTS,
  )?;
  
  let api_holder = &mut ctx.accounts.api_holder;
  api_holder.auth = ctx.accounts.auth.key();
  api_holder.owner = ctx.accounts.owner.key();
  api_holder.bump = ctx.bumps.api_holder;
  api_holder.vault = ctx.accounts.api_holder_vault.key();
  api_holder.vault_bump = ctx.bumps.api_holder_vault;
  api_holder.is_active = true;
  api_holder.fee_percentage = fee_percentage;
  api_holder.created_at = Clock::get()?.unix_timestamp;
  api_holder._reserved = [0u8; 32];
  
  emit_cpi!(ApiHolderInitialized {
        api_holder: api_holder.key(),
        auth: api_holder.auth,
        owner: api_holder.owner,
        fee_percentage,
        is_active: true,
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct InitApiHolder<'info> {
  #[account(mut)]
  pub owner: Signer<'info>,
  
  /// The Privy-managed wallet being registered as this holder's
  /// `auth`. Already exists off-chain; not required to sign here.
  /// CHECK: only ever credited lamports, never read as typed data —
  /// its role as `auth` is enforced later via `ApiHolder::require_auth`.
  #[account(mut)]
  pub auth: SystemAccount<'info>,
  
  #[account(
        init,
        payer = owner,
        space = 8 + ApiHolder::INIT_SPACE,
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  /// Plain system-owned PDA holding native SOL, no data. Not `init`'d —
  /// a System Program transfer to a fresh pubkey creates it directly.
  /// CHECK: address is fully determined by the seeds below; this
  /// account is never deserialized, only credited/debited lamports.
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}