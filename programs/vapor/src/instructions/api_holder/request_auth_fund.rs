use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::constants::AUTH_GAS_LAMPORTS;
use crate::error::ErrorCode;
use crate::state::seeds::API_HOLDER_VAULT_SEED;
use crate::state::ApiHolder;

/// Lets the `auth` wallet top itself up from the holder's vault.
///
/// - If `auth` holds >= 50% of `AUTH_GAS_LAMPORTS`, nothing happens.
/// - Otherwise the vault transfers exactly the amount needed to bring
///   `auth` up to 150% of `AUTH_GAS_LAMPORTS`.
pub fn handler(ctx: Context<RequestAuthFund>) -> Result<()> {
  let auth_balance = ctx.accounts.auth.lamports();
  
  // 50% of the minimum is the refill trigger.
  let threshold = AUTH_GAS_LAMPORTS / 2;
  if auth_balance >= threshold {
    return Ok(());
  }
  
  // Refill target: 150% of the minimum.
  let target = AUTH_GAS_LAMPORTS.saturating_add(AUTH_GAS_LAMPORTS / 2);
  let amount = target.saturating_sub(auth_balance);
  
  // The vault is a data-less system account, so it must remain rent-exempt
  // (or be drained to zero) after the transfer. Never dip below rent minimum.
  let rent_min = Rent::get()?.minimum_balance(0);
  let spendable = ctx
    .accounts
    .api_holder_vault
    .lamports()
    .saturating_sub(rent_min);
  require!(spendable >= amount, ErrorCode::InsufficientVaultFunds);
  
  // vault -> auth, signed by the vault PDA.
  let api_holder_key = ctx.accounts.api_holder.key();
  let vault_bump = [ctx.accounts.api_holder.vault_bump];
  let signer_seeds: &[&[&[u8]]] = &[&[
    API_HOLDER_VAULT_SEED,
    api_holder_key.as_ref(),
    &vault_bump,
  ]];
  
  system_program::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.system_program.key(),
      system_program::Transfer {
        from: ctx.accounts.api_holder_vault.to_account_info(),
        to: ctx.accounts.auth.to_account_info(),
      },
      signer_seeds,
    ),
    amount,
  )?;
  
  Ok(())
}

#[derive(Accounts)]
pub struct RequestAuthFund<'info> {
  /// The Privy-managed wallet registered as this holder's `auth`.
  #[account(mut)]
  pub auth: Signer<'info>,
  
  #[account(
        seeds = [ApiHolder::SEED_PREFIX, api_holder.owner.as_ref()],
        bump = api_holder.bump,
        has_one = auth,
  )]
  pub api_holder: Account<'info, ApiHolder>,
  
  /// Plain system-owned PDA holding native SOL, no data.
  /// CHECK: address is fully determined by the seeds below; only debited
  /// via a System Program transfer signed with the vault seeds.
  #[account(
        mut,
        seeds = [API_HOLDER_VAULT_SEED, api_holder.key().as_ref()],
        bump = api_holder.vault_bump,
  )]
  pub api_holder_vault: SystemAccount<'info>,
  
  pub system_program: Program<'info, System>,
}