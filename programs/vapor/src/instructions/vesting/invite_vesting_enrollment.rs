use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::VestingEnrollmentInvited;
use crate::state::seeds::{VESTING_ENROLLMENT_SEED, VESTING_VAULT_SEED};
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<InviteVestingEnrollment>, invitee: Pubkey) -> Result<()> {
  let policy = &mut ctx.accounts.policy;

  anchor_lang::system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      anchor_lang::system_program::Transfer {
        from: ctx.accounts.maker.to_account_info(),
        to: ctx.accounts.vesting_vault.to_account_info(),
      },
      &[&[VESTING_VAULT_SEED, policy.key().as_ref(), &[ctx.bumps.vesting_vault]]],
    ),
    policy.total_amount,
  )?;

  let enrollment = &mut ctx.accounts.enrollment;
  enrollment.policy = policy.key();
  enrollment.invitee = invitee;
  enrollment.claimed_amount = 0;
  enrollment.next_tranche_index = 0;
  enrollment.claim_ceiling = None;
  enrollment.invited_at = Clock::get()?.unix_timestamp;
  enrollment.bump = ctx.bumps.enrollment;

  // If policy is non-cancellable, auto-accept immediately
  if !policy.cancellable {
    enrollment.status = EnrollmentStatus::Active;
    enrollment.accepted_at = Some(enrollment.invited_at);
    policy.enrolled_invitee = policy.enrolled_invitee.checked_add(1).unwrap_or(policy.enrolled_invitee);
  } else {
    enrollment.status = EnrollmentStatus::Invited;
    enrollment.accepted_at = None;
  }

  emit!(VestingEnrollmentInvited {
        enrollment: enrollment.key(),
        policy: enrollment.policy,
        invitee,
    });

  Ok(())
}

#[derive(Accounts)]
#[instruction(invitee: Pubkey)]
pub struct InviteVestingEnrollment<'info> {
  // `policy` must come before `maker` below — Anchor resolves
  // constraints in declaration order, and `maker`'s constraint reads
  // policy.maker.
  #[account(
        mut,
        seeds = [VestingPolicy::SEED_PREFIX, policy.api_holder.as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump,
  )]
  pub vesting_vault: SystemAccount<'info>,
  
  #[account(mut, address = policy.maker @ ErrorCode::UnauthorizedMaker)]
  pub maker: Signer<'info>,
  
  #[account(
        init,
        payer = maker,
        space = 8 + VestingEnrollment::INIT_SPACE,
        seeds = [VESTING_ENROLLMENT_SEED, policy.key().as_ref(), invitee.as_ref()],
        bump,
  )]
  pub enrollment: Account<'info, VestingEnrollment>,
  
  pub system_program: Program<'info, System>,
}