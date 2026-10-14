use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::VestingEnrollmentCancelled;
use crate::instructions::vesting_helper::compute_cancellation_sweep;
use crate::state::seeds::VESTING_VAULT_SEED;
use crate::state::{EnrollmentStatus, VestingEnrollment, VestingPolicy};

pub fn handler(ctx: Context<CancelVestingEnrollmentToken>) -> Result<()> {
  require!(
        !ctx.accounts.policy.is_native_sol(),
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
    let policy_api_holder = policy.api_holder;
    let policy_id_bytes = policy.id.clone();
    let policy_bump = policy.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
      VestingPolicy::SEED_PREFIX,
      policy_api_holder.as_ref(),
      policy_id_bytes.as_bytes(),
      &[policy_bump],
    ]];
    
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
      swept,
      ctx.accounts.mint.decimals,
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
pub struct CancelVestingEnrollmentToken<'info> {
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