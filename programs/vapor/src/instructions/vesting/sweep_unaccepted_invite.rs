use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::INVITE_EXPIRY_SECONDS;
use crate::error::ErrorCode;
use crate::events::VestingEnrollmentSwept;
use crate::state::seeds::VESTING_VAULT_SEED;
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<SweepUnacceptedInvite>) -> Result<()> {
  require!(
        ctx.accounts.policy.is_native_sol(),
        ErrorCode::VestingMintMismatch
    );

  let enrollment = &ctx.accounts.enrollment;
  require!(
        enrollment.status == EnrollmentStatus::Invited,
        ErrorCode::InvalidEnrollmentStatus
    );

  let now = Clock::get()?.unix_timestamp;
  let expiry_time = enrollment.invited_at + INVITE_EXPIRY_SECONDS;
  require!(now >= expiry_time, ErrorCode::InviteNotExpired);

  let policy = &ctx.accounts.policy;
  let policy_key = policy.key();
  let vault_bump = policy.vault_bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[VESTING_VAULT_SEED, policy_key.as_ref(), &[vault_bump]]];

  // Sweep the full amount back to maker
  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.vesting_vault.to_account_info(),
        to: ctx.accounts.maker.to_account_info(),
      },
      signer_seeds,
    ),
    policy.total_amount,
  )?;

  let enrollment = &mut ctx.accounts.enrollment;
  enrollment.status = EnrollmentStatus::Cancelled;
  enrollment.claim_ceiling = Some(0); // No tranches can be claimed

  let policy = &mut ctx.accounts.policy;
  policy.enrolled_invitee = policy.enrolled_invitee.saturating_sub(1);

  emit!(VestingEnrollmentSwept {
        enrollment: enrollment.key(),
        policy: enrollment.policy,
        invitee: enrollment.invitee,
        swept_to_maker: policy.total_amount,
    });

  Ok(())
}

#[derive(Accounts)]
pub struct SweepUnacceptedInvite<'info> {
  // `policy` must come before `maker` — Anchor resolves constraints in
  // declaration order, and `maker`'s constraint reads policy.maker.
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
        close = maker,
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
