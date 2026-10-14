use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::constants::{MAX_ID_LEN, MAX_VESTING_TRANCHES};
use crate::error::ErrorCode;
use crate::events::VestingPolicyCreated;
use crate::instructions::vesting_helper::validate_tranches;
use crate::state::seeds::VESTING_VAULT_SEED;
use crate::state::{ApiHolder, Tranche, VestingPolicy};

/// `maker`-signed, not `auth`-gated — see the doc comment on
/// `VestingPolicy` for why: this is whoever actually funds the grant,
/// which may not be the api_holder's own operational key at all.
pub fn handler(
  ctx: Context<CreateVestingPolicyToken>,
  id: String,
  tranches: Vec<Tranche>,
  cancellable: bool,
  cancellable_after_tranche_index: Option<u8>,
  min_hard_locked_amount: u64,
) -> Result<()> {
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
  
  let policy = &mut ctx.accounts.policy;
  policy.id = id;
  policy.api_holder = ctx.accounts.api_holder.key();
  policy.maker = ctx.accounts.maker.key();
  policy.mint = ctx.accounts.mint.key();
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
pub struct CreateVestingPolicyToken<'info> {
  #[account(mut)]
  pub maker: Signer<'info>,
  
  /// Scoping only — namespaces this policy's PDA under a business, but
  /// doesn't authorize anything here; `maker`'s signature is what does.
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  pub mint: InterfaceAccount<'info, Mint>,
  
  #[account(
        init,
        payer = maker,
        space = 8 + VestingPolicy::INIT_SPACE,
        seeds = [VestingPolicy::SEED_PREFIX, api_holder.key().as_ref(), id.as_bytes()],
        bump,
  )]
  pub policy: Account<'info, VestingPolicy>,
  
  #[account(
        init,
        payer = maker,
        seeds = [VESTING_VAULT_SEED, policy.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = policy,
        token::token_program = token_program,
  )]
  pub vesting_vault: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}