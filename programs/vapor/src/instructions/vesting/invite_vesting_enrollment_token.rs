use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::VestingEnrollmentInvited;
use crate::state::seeds::{VESTING_ENROLLMENT_SEED, VESTING_VAULT_SEED};
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<InviteVestingEnrollmentToken>, invitee: Pubkey) -> Result<()> {
  let policy = &mut ctx.accounts.policy;

  require!(
        !policy.is_native_sol(),
        ErrorCode::VestingMintMismatch
    );

  transfer_checked(
    CpiContext::new(
      ctx.accounts.token_program.key(),
      TransferChecked {
        from: ctx.accounts.maker_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.vesting_vault.to_account_info(),
        authority: ctx.accounts.maker.to_account_info(),
      },
    ),
    policy.total_amount,
    ctx.accounts.mint.decimals,
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
pub struct InviteVestingEnrollmentToken<'info> {
  #[account(
        mut,
        seeds = [VestingPolicy::SEED_PREFIX, policy.api_holder.as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump = policy.vault_bump,
  )]
  pub vesting_vault: InterfaceAccount<'info, TokenAccount>,
  
  #[account(mut, address = policy.maker @ ErrorCode::UnauthorizedMaker)]
  pub maker: Signer<'info>,
  
  #[account(address = policy.mint @ ErrorCode::VestingMintMismatch)]
  pub mint: InterfaceAccount<'info, Mint>,
  
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
  )]
  pub maker_token_account: InterfaceAccount<'info, TokenAccount>,
  
  #[account(
        init,
        payer = maker,
        space = 8 + VestingEnrollment::INIT_SPACE,
        seeds = [VESTING_ENROLLMENT_SEED, policy.key().as_ref(), invitee.as_ref()],
        bump,
  )]
  pub enrollment: Account<'info, VestingEnrollment>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}
