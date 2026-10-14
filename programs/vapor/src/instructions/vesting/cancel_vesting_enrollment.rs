use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::ErrorCode;
use crate::events::VestingEnrollmentCancelled;
use crate::instructions::vesting_helper::compute_cancellation_sweep;
use crate::state::seeds::VESTING_VAULT_SEED;
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<CancelVestingEnrollment>) -> Result<()> {
  require!(
        ctx.accounts.policy.is_native_sol(),
        ErrorCode::VestingMintMismatch
    );
  
  let policy = &ctx.accounts.policy;
  require!(policy.cancellable, ErrorCode::VestingNotCancellable);
  
  let now = Clock::get()?.unix_timestamp;
  if let Some(idx) = policy.cancellable_after_tranche_index {
    let gate = policy.tranches.get(idx as usize).ok_or(ErrorCode::CancellationGateNotMet)?;
    require!(gate.unlock_time <= now, ErrorCode::CancellationGateNotMet);
  }
  let vested_amount: u64 = policy
    .tranches
    .iter()
    .filter(|t| t.unlock_time <= now)
    .map(|t| t.amount)
    .sum();
  require!(
        vested_amount >= policy.min_hard_locked_amount,
        ErrorCode::CancellationGateNotMet
    );
  
  let (swept, new_ceiling) =
    compute_cancellation_sweep(&policy.tranches, ctx.accounts.enrollment.next_tranche_index, now);
  
  if swept > 0 {
    let policy_key = policy.key();
    let vault_bump = policy.vault_bump;
    let signer_seeds: &[&[&[u8]]] =
      &[&[VESTING_VAULT_SEED, policy_key.as_ref(), &[vault_bump]]];

    system_program::transfer(
      CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        system_program::Transfer {
          from: ctx.accounts.vesting_vault.to_account_info(),
          to: ctx.accounts.maker.to_account_info(),
        },
        signer_seeds,
      ),
      swept,
    )?;
  }
  
  let enrollment = &mut ctx.accounts.enrollment;
  enrollment.status = EnrollmentStatus::Cancelled;
  enrollment.claim_ceiling = Some(new_ceiling);

  let policy = &mut ctx.accounts.policy;
  policy.enrolled_invitee = policy.enrolled_invitee.saturating_sub(1);
  
  emit!(VestingEnrollmentCancelled {
        enrollment: enrollment.key(),
        swept_to_maker: swept,
        claim_ceiling: new_ceiling,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct CancelVestingEnrollment<'info> {
  // `policy` must come before `maker` — same ordering reason as
  // cancel_vesting_enrollment.rs.
  #[account(
        mut,
        seeds = [VestingPolicy::SEED_PREFIX, policy.api_holder.as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  #[account(mut, address = policy.maker @ ErrorCode::UnauthorizedMaker)]
  pub maker: Signer<'info>,
  
  #[account(
        mut,
        seeds = [crate::state::seeds::VESTING_ENROLLMENT_SEED, policy.key().as_ref(), enrollment.invitee.as_ref()],
        bump = enrollment.bump,
  )]
  pub enrollment: Account<'info, VestingEnrollment>,
  
  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump = policy.vault_bump,
  )]
  pub vesting_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}