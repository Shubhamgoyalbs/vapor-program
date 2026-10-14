use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::events::Deposited;
use crate::state::seeds::API_HOLDER_VAULT_SEED;
use crate::state::ApiHolder;

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_owner(&ctx.accounts.owner.key())?;
  
  system_program::transfer(
    CpiContext::new(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.owner.to_account_info(),
        to: ctx.accounts.api_holder_vault.to_account_info(),
      },
    ),
    amount,
  )?;
  
  emit_cpi!(Deposited {
        api_holder: ctx.accounts.api_holder.key(),
        amount,
        new_vault_balance: ctx.accounts.api_holder_vault.lamports(),
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct Deposit<'info> {
  #[account(mut)]
  pub owner: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  /// CHECK: address fixed by seeds/bump against the value stored on
  /// `api_holder`; never deserialized, only credited lamports.
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}