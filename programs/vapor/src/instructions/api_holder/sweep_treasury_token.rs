use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
  transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::TREASURY_ADMIN;
use crate::state::seeds::TREASURY_SEED;

#[derive(Accounts)]
pub struct SweepTreasuryToken<'info> {
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

  #[account(
    mut,
    constraint = treasury_token_account.owner == treasury.key() @ crate::error::ErrorCode::TokenAccountOwnerMismatch
  )]
  pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,

  #[account(
    mut,
    constraint = admin_token_account.owner == treasury_admin.key() @ crate::error::ErrorCode::TokenAccountOwnerMismatch
  )]
  pub admin_token_account: InterfaceAccount<'info, TokenAccount>,

  pub mint: InterfaceAccount<'info, Mint>,

  pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<SweepTreasuryToken>, amount: u64) -> Result<()> {
  let treasury = &ctx.accounts.treasury;
  let treasury_token_account = &ctx.accounts.treasury_token_account;
  let admin_token_account = &ctx.accounts.admin_token_account;
  let mint = &ctx.accounts.mint;
  let bump = ctx.bumps.treasury;

  let signer_seeds: &[&[&[u8]]] = &[&[
    TREASURY_SEED,
    &[bump],
  ]];

  let decimals = mint.decimals;

  transfer_checked(
    CpiContext::new_with_signer(
      ctx.accounts.token_program.key(),
      TransferChecked {
        from: treasury_token_account.to_account_info(),
        mint: mint.to_account_info(),
        to: admin_token_account.to_account_info(),
        authority: treasury.to_account_info(),
      },
      signer_seeds,
    ),
    amount,
    decimals,
  )?;

  Ok(())
}
