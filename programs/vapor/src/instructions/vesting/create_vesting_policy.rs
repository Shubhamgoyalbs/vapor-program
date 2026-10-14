use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::{MAX_ID_LEN, MIN_VAULT_LAMPORTS, MAX_VESTING_TRANCHES, NATIVE_SOL_MINT_SENTINEL};
use crate::error::ErrorCode;
use crate::events::VestingPolicyCreated;
use crate::instructions::vesting_helper::validate_tranches;
use crate::state::seeds::{API_HOLDER_VAULT_SEED, VESTING_VAULT_SEED};
use crate::state::{ApiHolder, Tranche, VestingPolicy};

pub fn handler(
  ctx: Context<CreateVestingPolicy>,
  id: String,
  tranches: Vec<Tranche>,
  cancellable: bool,
  cancellable_after_tranche_index: Option<u8>,
  min_hard_locked_amount: u64,
) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_auth(&ctx.accounts.auth.key())?;
  
  require!(id.len() <= MAX_ID_LEN, ErrorCode::IdTooLong);
  let total_amount = validate_tranches(&tranches)?;
  
  let tranche_count = tranches.iter().count() as u8;
  if cancellable {
    if let Some(idx) = cancellable_after_tranche_index {
      require!(idx < tranche_count, ErrorCode::InvalidCancellationTerms);
    }
  } else {
    require!(
            cancellable_after_tranche_index.is_none(),
            ErrorCode::InvalidCancellationTerms
        );
  }
  require!(
      min_hard_locked_amount <= total_amount,
      ErrorCode::InvalidHardLockedAmount
  );
  
  let policy_key = ctx.accounts.policy.key();
  let seeds: &[&[u8]] = &[VESTING_VAULT_SEED, policy_key.as_ref(), &[ctx.bumps.vesting_vault]];
  let rent = Rent::get()?.minimum_balance(0);
  system_program::create_account(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::CreateAccount {
        from: ctx.accounts.auth.to_account_info(),
        to: ctx.accounts.vesting_vault.to_account_info(),
      },
      &[seeds],
    ),
    rent,
    0,
    &system_program::ID,
  )?;
  
  let rent_reimbursement = ctx.accounts.policy.to_account_info().lamports() +  rent;
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
  policy.api_holder = ctx.accounts.api_holder.key();
  policy.maker = ctx.accounts.maker.key();
  policy.mint = NATIVE_SOL_MINT_SENTINEL;
  policy.total_amount = total_amount;
  policy.tranches = tranches;
  policy.cancellable = cancellable;
  policy.cancellable_after_tranche_index = cancellable_after_tranche_index;
  policy.min_hard_locked_amount = min_hard_locked_amount;
  policy.enrolled_invitee = 0;
  policy.vault = ctx.accounts.vesting_vault.key();
  policy.vault_bump = ctx.bumps.vesting_vault;
  policy.created_at = Clock::get()?.unix_timestamp;
  policy.bump = ctx.bumps.policy;
  policy._reserved = [0u8; 16];
  
  emit!(VestingPolicyCreated {
        policy: policy.key(),
        api_holder: policy.api_holder,
        maker: policy.maker,
        mint: policy.mint,
        total_amount,
        cancellable,
    });
  
  Ok(())
}

#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreateVestingPolicy<'info> {
  #[account(mut)]
  pub auth: Signer<'info>,
  
  #[account(mut)]
  pub maker: SystemAccount<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
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
  
  #[account(
        init,
        payer = maker,
        space = 8 + VestingPolicy::INIT_SPACE,
        seeds = [VestingPolicy::SEED_PREFIX, api_holder.key().as_ref(), id.as_bytes()],
        bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  /// System-owned PDA holding native SOL — the vault's own seeds will
  /// sign its outgoing transfers later (claim / cancel), same pattern
  /// as every other SOL-rail vault in this program.
  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump,
  )]
  pub vesting_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}