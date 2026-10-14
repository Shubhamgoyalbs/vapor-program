use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::TREASURY_ADMIN;
use crate::state::seeds::TREASURY_SEED;

#[derive(Accounts)]
pub struct SweepTreasury<'info> {
  #[account(
    mut,
    constraint = treasury_admin.key() == TREASURY_ADMIN @ crate::error::ErrorCode::UnauthorizedAdmin
  )]
  pub treasury_admin: Signer<'info>,

  #[account(
    mut,
    seeds = [TREASURY_SEED],
    bump,
  )]
  pub treasury: SystemAccount<'info>,

  pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SweepTreasury>, amount: u64) -> Result<()> {
  let treasury = &ctx.accounts.treasury;
  let treasury_admin = &ctx.accounts.treasury_admin;
  let bump = ctx.bumps.treasury;

  let signer_seeds: &[&[&[u8]]] = &[&[
    TREASURY_SEED,
    &[bump],
  ]];

  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: treasury.to_account_info(),
        to: treasury_admin.to_account_info(),
      },
      signer_seeds,
    ),
    amount,
  )?;

  Ok(())
}
