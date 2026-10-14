use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::VestingTrancheClaimed;
use crate::instructions::vesting_helper::claimable_amount;
use crate::send;
use crate::state::seeds::{API_HOLDER_VAULT_SEED, TREASURY_SEED, VESTING_VAULT_SEED};
use crate::state::{ApiHolder, EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<ClaimVestingTranche>) -> Result<()> {
  ctx.accounts.api_holder
    .require_auth(
      ctx.accounts.auth.key
    )?;
  
  require!(
        ctx.accounts.policy.is_native_sol(),
        ErrorCode::VestingMintMismatch
    );
  
  let enrollment = &ctx.accounts.enrollment;
  require!(
        matches!(
            enrollment.status,
            EnrollmentStatus::Active | EnrollmentStatus::Cancelled
        ),
        ErrorCode::EnrollmentNotClaimable
    );
  
  let now = Clock::get()?.unix_timestamp;
  let (amount, new_next_index) = claimable_amount(
    &ctx.accounts.policy.tranches,
    enrollment.next_tranche_index,
    enrollment.claim_ceiling,
    now,
  );
  require!(amount > 0, ErrorCode::NothingVestedYet);
  let tranches_claimed = new_next_index - enrollment.next_tranche_index;
  
  // Unlike the token path, there's no SPL "authority" to delegate to —
  // the vault is a plain lamport PDA, so it signs for itself (not
  // `policy`'s seeds).
  let policy_key = ctx.accounts.policy.key();
  let vault_bump = ctx.accounts.policy.vault_bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[VESTING_VAULT_SEED, policy_key.as_ref(), &[vault_bump]]];
  
  send(
    &ctx.accounts.system_program,
    ctx.accounts.vesting_vault.to_account_info(),
    signer_seeds,
    ctx.accounts.api_holder_vault.to_account_info(),
    ctx.accounts.treasury.to_account_info(),
    ctx.accounts.invitee.to_account_info(),
    amount,
    ctx.accounts.api_holder.fee_percentage,
  )?;
  
  let enrollment = &mut ctx.accounts.enrollment;
  enrollment.claimed_amount = enrollment.claimed_amount.saturating_add(amount);
  enrollment.next_tranche_index = new_next_index;
  
  let total_tranches = ctx.accounts.policy.tranches.iter().count() as u8;
  let ceiling = enrollment.claim_ceiling.unwrap_or(total_tranches);
  if enrollment.next_tranche_index >= ceiling {
    enrollment.status = EnrollmentStatus::Completed;
    ctx.accounts.policy.enrolled_invitee =
      ctx.accounts.policy.enrolled_invitee.saturating_sub(1);
  }
  
  emit!(VestingTrancheClaimed {
        enrollment: enrollment.key(),
        policy: policy_key,
        amount,
        tranches_claimed,
        next_tranche_index: enrollment.next_tranche_index,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct ClaimVestingTranche<'info> {
  #[account(mut)]
  pub auth: Signer<'info>,
  
  #[account(mut)]
  pub invitee: SystemAccount<'info>,
  
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
        seeds = [VestingPolicy::SEED_PREFIX, api_holder.key().as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  #[account(
        mut,
        seeds = [crate::state::seeds::VESTING_ENROLLMENT_SEED, policy.key().as_ref(), invitee.key().as_ref()],
        bump = enrollment.bump,
  )]
  pub enrollment: Account<'info, VestingEnrollment>,
  
  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump = policy.vault_bump,
  )]
  pub vesting_vault: SystemAccount<'info>,
  
  #[account(
    mut,
    seeds = [TREASURY_SEED],
    bump,
  )]
  pub treasury: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}