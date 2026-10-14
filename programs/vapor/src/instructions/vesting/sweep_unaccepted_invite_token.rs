use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::constants::INVITE_EXPIRY_SECONDS;
use crate::error::ErrorCode;
use crate::events::VestingEnrollmentSwept;
use crate::state::seeds::VESTING_VAULT_SEED;
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<SweepUnacceptedInviteToken>) -> Result<()> {
  require!(
        !ctx.accounts.policy.is_native_sol(),
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
  let policy_api_holder = policy.api_holder;
  let policy_id_bytes = policy.id.clone();
  let policy_bump = policy.bump;
  let signer_seeds: &[&[&[u8]]] = &[&[
    VestingPolicy::SEED_PREFIX,
    policy_api_holder.as_ref(),
    policy_id_bytes.as_bytes(),
    &[policy_bump],
  ]];

  // Sweep the full amount back to maker
  transfer_checked(
    CpiContext::new_with_signer(
      ctx.accounts.token_program.key(),
      TransferChecked {
        from: ctx.accounts.vesting_vault.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.maker_token_account.to_account_info(),
        authority: ctx.accounts.policy.to_account_info(),
      },
      signer_seeds,
    ),
    policy.total_amount,
    ctx.accounts.mint.decimals,
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
pub struct SweepUnacceptedInviteToken<'info> {
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

  #[account(address = policy.mint @ ErrorCode::VestingMintMismatch)]
  pub mint: InterfaceAccount<'info, Mint>,

  #[account(
        mut,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump = policy.vault_bump,
  )]
  pub vesting_vault: InterfaceAccount<'info, TokenAccount>,

  #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
  )]
  pub maker_token_account: InterfaceAccount<'info, TokenAccount>,

  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}
