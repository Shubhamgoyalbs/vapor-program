use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::error::ErrorCode;
use crate::events::VestingTrancheClaimed;
use crate::instructions::vesting_helper::claimable_amount;
use crate::send_token;
use crate::state::seeds::{VESTING_VAULT_SEED, TREASURY_SEED};
use crate::state::{ApiHolder, EnrollmentStatus, VestingEnrollment, VestingPolicy};

/// Claims every tranche that's vested since the last claim in one call,
/// not just the next single one — avoids forcing a separate transaction
/// per tranche for someone who hasn't claimed in a while.
pub fn handler(ctx: Context<ClaimVestingTrancheToken>) -> Result<()> {
  ctx.accounts.api_holder
    .require_auth(
      ctx.accounts.auth.key
    )?;
  
  require!(
        !ctx.accounts.policy.is_native_sol(),
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
  
  let policy_api_holder = ctx.accounts.policy.api_holder;
  let policy_id_bytes = ctx.accounts.policy.id.clone();
  let policy_bump = ctx.accounts.policy.bump;
  let signer_seeds: &[&[&[u8]]] = &[&[
    VestingPolicy::SEED_PREFIX,
    policy_api_holder.as_ref(),
    policy_id_bytes.as_bytes(),
    &[policy_bump],
  ]];
  
  send_token(
    &ctx.accounts.token_program,
    &ctx.accounts.mint,
    &ctx.accounts.vesting_vault,
    ctx.accounts.policy.to_account_info(),
    signer_seeds,
    &ctx.accounts.api_holder_vault,
    &ctx.accounts.treasury_token,
    &ctx.accounts.invitee_token_account,
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
        policy: ctx.accounts.policy.key(),
        amount,
        tranches_claimed,
        next_tranche_index: enrollment.next_tranche_index,
    });
  
  Ok(())
}

#[derive(Accounts)]
pub struct ClaimVestingTrancheToken<'info> {
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
        seeds = [VestingPolicy::SEED_PREFIX, api_holder.key().as_ref(), policy.id.as_bytes()],
        bump = policy.bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  // Seeds use the live signer's own key, not enrollment.invitee.
  #[account(
        mut,
        seeds = [crate::state::seeds::VESTING_ENROLLMENT_SEED, policy.key().as_ref(), invitee.key().as_ref()],
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
    mut,
    seeds = [TREASURY_SEED],
    bump,
  )]
  pub treasury: SystemAccount<'info>,

  #[account(
        init_if_needed,
        payer = invitee,
        associated_token::mint = mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
  )]
  pub treasury_token: InterfaceAccount<'info, TokenAccount>,

  #[account(
        init_if_needed,
        payer = invitee,
        associated_token::mint = mint,
        associated_token::authority = invitee,
        associated_token::token_program = token_program,
  )]
  pub invitee_token_account: InterfaceAccount<'info, TokenAccount>,

  #[account(
        init_if_needed,
        payer = invitee,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_vault: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}