use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::{
  transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::{PERCENTAGE_PRECISION, PROGRAM_FEE_PERCENTAGE};
use crate::error::ErrorCode;
use crate::state::SplitRecipient;

/// Compute `amount * bps / PERCENTAGE_PRECISION` safely via u128
/// to avoid overflow on large token / lamport amounts.
#[inline(always)]
pub fn fee_of(amount: u64, bps: u32) -> u64 {
  (amount as u128 * bps as u128 / PERCENTAGE_PRECISION as u128) as u64
}

#[allow(clippy::too_many_arguments)]
pub fn send<'info>(
  system_program_account: &Program<'info, System>,
  vault: AccountInfo<'info>,
  signer_seeds: &[&[&[u8]]],
  fee_destination: AccountInfo<'info>,
  treasury: AccountInfo<'info>,
  to: AccountInfo<'info>,
  amount: u64,
  api_fee_percentage: u32,
) -> Result<()> {
  if amount == 0 {
    return Ok(());
  }

  let program_fee_amount = fee_of(amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(amount, api_fee_percentage);
  let net_amount = amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  let sys = system_program_account.to_account_info();

  if program_fee_amount > 0 {
    system_program::transfer(
      CpiContext::new_with_signer(
        sys.key(),
        system_program::Transfer {
          from: vault.clone(),
          to: treasury,
        },
        signer_seeds,
      ),
      program_fee_amount,
    )?;
  }

  if api_fee_amount > 0 {
    system_program::transfer(
      CpiContext::new_with_signer(
        sys.key(),
        system_program::Transfer {
          from: vault.clone(),
          to: fee_destination,
        },
        signer_seeds,
      ),
      api_fee_amount,
    )?;
  }

  if net_amount > 0 {
    system_program::transfer(
      CpiContext::new_with_signer(
        sys.key(),
        system_program::Transfer {
          from: vault,
          to,
        },
        signer_seeds,
      ),
      net_amount,
    )?;
  }

  Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn send_token<'info>(
  token_program: &Interface<'info, TokenInterface>,
  mint: &InterfaceAccount<'info, Mint>,
  vault: &InterfaceAccount<'info, TokenAccount>,
  vault_authority: AccountInfo<'info>,
  signer_seeds: &[&[&[u8]]],
  fee_destination: &InterfaceAccount<'info, TokenAccount>,
  treasury: &InterfaceAccount<'info, TokenAccount>,
  to: &InterfaceAccount<'info, TokenAccount>,
  amount: u64,
  api_fee_percentage: u32,
) -> Result<()> {
  if amount == 0 {
    return Ok(());
  }

  let program_fee_amount = fee_of(amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(amount, api_fee_percentage);
  let net_amount = amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  let tok = token_program.to_account_info();
  let decimals = mint.decimals;

  if program_fee_amount > 0 {
    transfer_checked(
      CpiContext::new_with_signer(
        tok.key(),
        TransferChecked {
          from: vault.to_account_info(),
          mint: mint.to_account_info(),
          to: treasury.to_account_info(),
          authority: vault_authority.clone(),
        },
        signer_seeds,
      ),
      program_fee_amount,
      decimals,
    )?;
  }

  if api_fee_amount > 0 {
    transfer_checked(
      CpiContext::new_with_signer(
        tok.key(),
        TransferChecked {
          from: vault.to_account_info(),
          mint: mint.to_account_info(),
          to: fee_destination.to_account_info(),
          authority: vault_authority.clone(),
        },
        signer_seeds,
      ),
      api_fee_amount,
      decimals,
    )?;
  }

  if net_amount > 0 {
    transfer_checked(
      CpiContext::new_with_signer(
        tok.key(),
        TransferChecked {
          from: vault.to_account_info(),
          mint: mint.to_account_info(),
          to: to.to_account_info(),
          authority: vault_authority,
        },
        signer_seeds,
      ),
      net_amount,
      decimals,
    )?;
  }

  Ok(())
}

pub fn pay<'info>(
  system_program: &Pubkey,
  vault: &AccountInfo<'info>,
  to: &AccountInfo<'info>,
  amount: u64,
) -> Result<()> {
  if amount == 0 {
    return Ok(());
  }
  system_program::transfer(
    CpiContext::new(
      *system_program,
      system_program::Transfer {
        from: vault.clone(),
        to: to.clone(),
      },
    ),
    amount,
  )
}

#[allow(clippy::too_many_arguments)]
pub fn pay_token<'info>(
  token_program: &Pubkey,
  from: &AccountInfo<'info>,
  mint: &AccountInfo<'info>,
  to: &AccountInfo<'info>,
  authority: &AccountInfo<'info>,
  amount: u64,
  decimals: u8,
) -> Result<()> {
  if amount == 0 {
    return Ok(());
  }
  transfer_checked(
    CpiContext::new(
      *token_program,
      TransferChecked {
        from: from.clone(),
        mint: mint.clone(),
        to: to.clone(),
        authority: authority.clone(),
      },
    ),
    amount,
    decimals,
  )
}

/// Owner field of a token account (bytes 32..64). The base layout is the
/// same for Token and Token-2022, extensions only append after byte 165.
pub fn token_account_owner(acc: &AccountInfo) -> Result<Pubkey> {
  let data = acc.try_borrow_data()?;
  require!(data.len() >= 165, ErrorCode::InvalidTokenAccount);
  Ok(Pubkey::new_from_array(data[32..64].try_into().unwrap()))
}

#[allow(clippy::too_many_arguments)]
pub fn distribute<'info>(
  system_program: &Program<'info, System>,
  vault: AccountInfo<'info>,
  signer_seeds: &[&[&[u8]]],
  fee_destination: AccountInfo<'info>,
  treasury: AccountInfo<'info>,
  split_recipients: &Vec<SplitRecipient>,
  remaining_accounts: &[AccountInfo<'info>],
  total_amount: u64,
  api_fee_percentage: u32,
) -> Result<()> {
  if total_amount == 0 {
    return Ok(());
  }

  let sys = system_program;

  // ---- fees ----
  let program_fee_amount = fee_of(total_amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(total_amount, api_fee_percentage);
  let net_amount = total_amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  // ---- net split ----
  let recipients = &split_recipients;
  if recipients.is_empty() {
    // todo
  }

  require!(
      remaining_accounts.len() >= recipients.len(),
      ErrorCode::MissingRecipientAccounts
    );

  let last = recipients.len() - 1;
  let mut distributed: u64 = 0;

  for (i, (r, acc)) in recipients.iter().zip(remaining_accounts).enumerate() {
    require_keys_eq!(r.recipient, acc.key(), ErrorCode::RecipientAccountMismatch);

    let share = if i == last {
      net_amount.saturating_sub(distributed) // remainder, so no dust
    } else {
      (net_amount as u128 * r.percentage as u128 / PERCENTAGE_PRECISION as u128) as u64
    };
    distributed = distributed.saturating_add(share);

    send(
      sys,
      vault.clone(),
      signer_seeds,
      fee_destination.clone(),
      treasury.clone(),
      acc.clone(),
      share,
      api_fee_percentage,
    )?;
  }
  Ok(())
}

/// Distribute from a signer (non-PDA) account - uses pay instead of send
#[allow(clippy::too_many_arguments)]
pub fn distribute_from_signer<'info>(
  system_program: &Pubkey,
  vault: &AccountInfo<'info>,
  fee_destination: &AccountInfo<'info>,
  treasury: &AccountInfo<'info>,
  split_recipients: &Vec<SplitRecipient>,
  remaining_accounts: &[AccountInfo<'info>],
  total_amount: u64,
  api_fee_percentage: u32,
) -> Result<()> {
  if total_amount == 0 {
    return Ok(());
  }

  // ---- fees ----
  let program_fee_amount = fee_of(total_amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(total_amount, api_fee_percentage);
  let net_amount = total_amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  // Pay fees using pay (no signer_seeds needed for signer)
  pay(system_program, vault, treasury, program_fee_amount)?;
  pay(system_program, vault, fee_destination, api_fee_amount)?;

  // ---- net split ----
  let recipients = &split_recipients;
  require!(!recipients.is_empty(), ErrorCode::EmptySplitRecipients);

  require!(
      remaining_accounts.len() >= recipients.len(),
      ErrorCode::MissingRecipientAccounts
    );

  let last = recipients.len() - 1;
  let mut distributed: u64 = 0;

  for (i, (r, acc)) in recipients.iter().zip(remaining_accounts).enumerate() {
    require_keys_eq!(r.recipient, acc.key(), ErrorCode::RecipientAccountMismatch);

    let share = if i == last {
      net_amount.saturating_sub(distributed) // remainder, so no dust
    } else {
      (net_amount as u128 * r.percentage as u128 / PERCENTAGE_PRECISION as u128) as u64
    };
    distributed = distributed.saturating_add(share);

    pay(system_program, vault, acc, share)?;
  }
  Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn distribute_token<'info>(
  token_program: &Interface<'info, TokenInterface>,
  mint: &InterfaceAccount<'info, Mint>,
  vault: &AccountInfo<'info>,
  vault_authority: &AccountInfo<'info>,
  split_recipients: Vec<SplitRecipient>,
  fee_destination: &AccountInfo<'info>,
  treasury: &AccountInfo<'info>,
  remaining_accounts: &[AccountInfo<'info>],
  total_amount: u64,
  api_fee_percentage: u32,
) -> Result<()> {
  if total_amount == 0 {
    return Ok(());
  }

  let tp = token_program.key();
  let mint_info = mint.to_account_info(); // one clone, reused below
  let decimals = mint.decimals;

  let program_fee_amount = fee_of(total_amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(total_amount, api_fee_percentage);
  let net_amount = total_amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  pay_token(&tp, vault, &mint_info, treasury, vault_authority, program_fee_amount, decimals)?;
  pay_token(&tp, vault, &mint_info, fee_destination, vault_authority, api_fee_amount, decimals)?;

  let recipients = &split_recipients;
  require!(!recipients.is_empty(), ErrorCode::EmptySplitRecipients);

  require!(
      remaining_accounts.len() >= recipients.len(),
      ErrorCode::MissingRecipientAccounts
    );

  let last = recipients.len() - 1;
  let mut distributed: u64 = 0;

  for (i, (r, ata)) in recipients.iter().zip(remaining_accounts).enumerate() {
    require_keys_eq!(
        token_account_owner(ata)?,
        r.recipient,
        ErrorCode::RecipientAccountMismatch
      );

    let share = if i == last {
      net_amount.saturating_sub(distributed)
    } else {
      (net_amount as u128 * r.percentage as u128 / PERCENTAGE_PRECISION as u128) as u64
    };
    distributed = distributed.saturating_add(share);

    pay_token(&tp, vault, &mint_info, ata, vault_authority, share, decimals)?;
  }
  Ok(())
}

/// Distribute tokens from a signer (non-PDA) account
#[allow(clippy::too_many_arguments)]
pub fn distribute_token_from_signer<'info>(
  token_program: &Pubkey,
  mint: &AccountInfo<'info>,
  vault: &AccountInfo<'info>,
  authority: &AccountInfo<'info>,
  split_recipients: &Vec<SplitRecipient>,
  fee_destination: &AccountInfo<'info>,
  treasury: &AccountInfo<'info>,
  remaining_accounts: &[AccountInfo<'info>],
  total_amount: u64,
  api_fee_percentage: u32,
  decimals: u8,
) -> Result<()> {
  if total_amount == 0 {
    return Ok(());
  }

  let program_fee_amount = fee_of(total_amount, PROGRAM_FEE_PERCENTAGE);
  let api_fee_amount = fee_of(total_amount, api_fee_percentage);
  let net_amount = total_amount
    .saturating_sub(program_fee_amount)
    .saturating_sub(api_fee_amount);

  pay_token(token_program, vault, mint, treasury, authority, program_fee_amount, decimals)?;
  pay_token(token_program, vault, mint, fee_destination, authority, api_fee_amount, decimals)?;

  let recipients = &split_recipients;
  require!(!recipients.is_empty(), ErrorCode::EmptySplitRecipients);

  require!(
      remaining_accounts.len() >= recipients.len(),
      ErrorCode::MissingRecipientAccounts
    );

  let last = recipients.len() - 1;
  let mut distributed: u64 = 0;

  for (i, (r, ata)) in recipients.iter().zip(remaining_accounts).enumerate() {
    require_keys_eq!(
        token_account_owner(ata)?,
        r.recipient,
        ErrorCode::RecipientAccountMismatch
      );

    let share = if i == last {
      net_amount.saturating_sub(distributed)
    } else {
      (net_amount as u128 * r.percentage as u128 / PERCENTAGE_PRECISION as u128) as u64
    };
    distributed = distributed.saturating_add(share);

    pay_token(token_program, vault, mint, ata, authority, share, decimals)?;
  }
  Ok(())
}