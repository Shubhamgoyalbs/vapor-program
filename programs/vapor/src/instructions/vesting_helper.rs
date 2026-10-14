use anchor_lang::prelude::*;

use crate::constants::MAX_VESTING_TRANCHES;
use crate::error::ErrorCode;
use crate::state::Tranche;

/// VESTING

/// Non-empty, no gaps (every `Some` before any `None`), strictly
/// increasing by `unlock_time`, every amount > 0. Returns the summed
/// total_amount.
pub fn validate_tranches(tranches: &[Tranche]) -> Result<u64> {
  require!(!tranches.is_empty(), ErrorCode::InvalidTrancheSchedule);
  require!(tranches.len() <= MAX_VESTING_TRANCHES, ErrorCode::TooManyTranches);

  let mut total: u64 = 0;
  let mut last_unlock: Option<i64> = None;
  
  for tranche in tranches.iter() {
    require!(tranche.amount > 0, ErrorCode::InvalidTrancheSchedule);
    if let Some(prev) = last_unlock {
      require!(tranche.unlock_time > prev, ErrorCode::InvalidTrancheSchedule);
    }
    last_unlock = Some(tranche.unlock_time);
    total = total
      .checked_add(tranche.amount)
      .ok_or(ErrorCode::InvalidTrancheSchedule)?;
  }
  
  Ok(total)
}

/// Sums every unclaimed, already-vested tranche starting at
/// `next_tranche_index`, stopping at the first not-yet-vested tranche or
/// at `claim_ceiling` (whichever comes first — tranches are monotonic,
/// so there's no "skipping ahead"). Returns (amount_to_claim,
/// new_next_tranche_index).
pub fn claimable_amount(
  tranches: &[Tranche],
  next_tranche_index: u8,
  claim_ceiling: Option<u8>,
  now: i64,
) -> (u64, u8) {
  let ceiling = claim_ceiling.unwrap_or(tranches.len() as u8);
  let mut amount: u64 = 0;
  let mut idx = next_tranche_index;
  
  while idx < ceiling && (idx as usize) < tranches.len() {
    match tranches.get(idx as usize) {
      Some(t) if t.unlock_time <= now => {
        amount = amount.saturating_add(t.amount);
        idx += 1;
      }
      _ => break,
    }
  }
  (amount, idx)
}

/// At cancellation: everything from `next_tranche_index` onward that's
/// already vested stays claimable; everything after that (monotonic, so
/// it's a contiguous tail) is unvested and gets swept back to `maker`.
/// Returns (amount_to_sweep, new_claim_ceiling).
pub fn compute_cancellation_sweep(
  tranches: &[Tranche],
  next_tranche_index: u8,
  now: i64,
) -> (u64, u8) {
  let mut idx = next_tranche_index as usize;
  while idx < tranches.len() {
    match tranches.get(idx) {
      Some(t) if t.unlock_time <= now => idx += 1,
      _ => break,
    }
  }
  let ceiling = idx as u8;
  let swept: u64 = tranches[idx..].iter().map(|t| t.amount).sum();
  (swept, ceiling)
}