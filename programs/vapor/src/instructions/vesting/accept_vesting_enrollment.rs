use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::VestingEnrollmentAccepted;
use crate::state::seeds::VESTING_ENROLLMENT_SEED;
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<AcceptVestingEnrollment>) -> Result<()> {
  let enrollment = &mut ctx.accounts.enrollment;
  require!(
        enrollment.status == EnrollmentStatus::Invited,
        ErrorCode::EnrollmentNotInvited
    );
  
  let now = Clock::get()?.unix_timestamp;
  enrollment.status = EnrollmentStatus::Active;
  enrollment.accepted_at = Some(now);
  
  emit!(VestingEnrollmentAccepted {
        enrollment: enrollment.key(),
        accepted_at: now,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct AcceptVestingEnrollment<'info> {
  pub invitee: Signer<'info>,
  
  // Seeds use the live signer's own key, not enrollment.invitee — that
  // alone is what proves this signer IS the invitee.
  #[account(
        mut,
        seeds = [VESTING_ENROLLMENT_SEED, enrollment.policy.as_ref(), invitee.key().as_ref()],
        bump = enrollment.bump,
  )]
  pub enrollment: Account<'info, VestingEnrollment>,
  
  #[account(address = enrollment.policy)]
  pub policy: Account<'info, VestingPolicy>,
}