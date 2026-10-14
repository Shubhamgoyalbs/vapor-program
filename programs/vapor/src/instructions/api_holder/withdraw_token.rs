use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::ErrorCode;
use crate::events::TokenWithdrawn;
use crate::state::ApiHolder;

/// Claims from the api_holder's own token vault for `mint` — an ATA
/// owned (authority) by the api_holder PDA, distinct from `vault` on
/// `ApiHolder` itself, which only ever holds native SOL. One of these
/// exists per mint the holder has ever collected a fee in; it's implicit
/// (derived via associated_token constraints below), never stored as a
/// field on `ApiHolder`.
pub fn handler(ctx: Context<WithdrawToken>, amount: u64) -> Result<()> {
  ctx.accounts
    .api_holder
    .require_owner(&ctx.accounts.owner.key())?;
  
  require!(
        ctx.accounts.api_holder_token_vault.amount >= amount,
        ErrorCode::InsufficientVaultTokenBalance
    );
  
  let owner_key = ctx.accounts.api_holder.owner;
  let bump = ctx.accounts.api_holder.bump;
  let signer_seeds: &[&[&[u8]]] =
    &[&[ApiHolder::SEED_PREFIX, owner_key.as_ref(), &[bump]]];
  
  transfer_checked(
    CpiContext::new_with_signer(
      ctx.accounts.token_program.key(),
      TransferChecked {
        from: ctx.accounts.api_holder_token_vault.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.owner_token_account.to_account_info(),
        authority: ctx.accounts.api_holder.to_account_info(),
      },
      signer_seeds,
    ),
    amount,
    ctx.accounts.mint.decimals,
  )?;
  
  emit_cpi!(TokenWithdrawn {
        api_holder: ctx.accounts.api_holder.key(),
        mint: ctx.accounts.mint.key(),
        amount,
        destination: ctx.accounts.owner_token_account.key(),
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct WithdrawToken<'info> {
  #[account(mut)]
  pub owner: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  pub mint: InterfaceAccount<'info, Mint>,
  
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = api_holder,
        associated_token::token_program = token_program,
  )]
  pub api_holder_token_vault: InterfaceAccount<'info, TokenAccount>,
  
  /// Created on demand if the owner doesn't already have an ATA for
  /// this mint — a missing destination account shouldn't be a reason
  /// to fail a claim.
  #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
  )]
  pub owner_token_account: InterfaceAccount<'info, TokenAccount>,
  
  pub token_program: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}