use anchor_lang::prelude::*;

use crate::constants::{MAX_ID_LEN, MAX_VESTING_TRANCHES, NATIVE_SOL_MINT_SENTINEL};
use crate::state::seeds::{VESTING_ENROLLMENT_SEED, VESTING_POLICY_SEED};

/// One per client-supplied policy id, scoped to an api_holder
/// (seeds: [VESTING_POLICY_SEED, api_holder, id]). Full `total_amount`
/// (the sum of every tranche) is escrowed up front at creation into
/// `vault` — tranches are never drip-funded.
///
/// `maker` is a genuinely separate signer from the api_holder's `auth`/
/// `owner` — whoever actually funds the grant (could be the business
/// owner, could be an outside investor/treasury wallet) signs and pays
/// for this directly; `api_holder` only scopes which business this
/// grant is organized under, it doesn't authorize anything here.
///
/// One escrowed pot, one beneficiary at a time: `enrolled_invitee`
/// blocks a second, different invite while one is pending or active,
/// and — importantly — is never cleared after a cancellation either.
/// Re-inviting on an already-cancelled policy would hand a new
/// beneficiary a tranche schedule the vault can no longer fully back
/// (the unvested portion was already swept back to `maker`), so a
/// cancelled policy is permanently done; `maker` creates a fresh policy
/// for a new beneficiary instead.
#[account]
#[derive(InitSpace)]
pub struct VestingPolicy {
  #[max_len(MAX_ID_LEN)]
  pub id: String,
  pub api_holder: Pubkey,
  pub maker: Pubkey,
  pub mint: Pubkey,
  /// Sum of every tranche's amount — computed at creation from
  /// `tranches`, not independently supplied.
  pub total_amount: u64,
  /// Sorted ascending by `unlock_time`, strictly increasing — enforced
  /// at creation. All-None past the active entries.
  #[max_len(MAX_VESTING_TRANCHES)]
  pub tranches: Vec<Tranche>,
  pub cancellable: bool,
  /// Cancellation requires tranche `cancellable_after_tranche_index`
  /// to have already unlocked. None + cancellable = true means no such
  /// gate — only `min_hard_locked_amount` applies. Always None when
  /// `cancellable` is false.
  pub cancellable_after_tranche_index: Option<u8>,
  /// Total vested amount (unlocked, whether claimed or not) that must
  /// already be reached before cancellation is permitted, on top of
  /// any `cancellable_after_tranche_index` gate.
  pub min_hard_locked_amount: u64,
  /// Number of active holders in this policy
  pub enrolled_invitee: u64,
  pub vault: Pubkey,
  pub vault_bump: u8,
  pub created_at: i64,
  pub bump: u8,
  pub _reserved: [u8; 16],
}

impl VestingPolicy {
  pub const SEED_PREFIX: &'static [u8] = VESTING_POLICY_SEED;
  
  /// True for a policy created via `create_vesting_policy_sol` — paid
  /// in plain lamports, not SPL/Token-2022 at all.
  pub fn is_native_sol(&self) -> bool {
    self.mint == NATIVE_SOL_MINT_SENTINEL
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq)]
pub struct Tranche {
  pub unlock_time: i64,
  pub amount: u64,
}

/// One per invited/enrolled person, own PDA rather than a Vec on the
/// policy (seeds: [VESTING_ENROLLMENT_SEED, policy, invitee]) — this is
/// the resolution to the spec's "one enrollee vs many" open question:
/// each invitation gets its own bounded-size account rather than an
/// unbounded Vec on the policy that would need periodic reallocation.
/// In practice, given `VestingPolicy.enrolled_invitee`'s one-at-a-time
/// rule, there's only ever one of these with real funds behind it per
/// policy — but keeping it a separate account (rather than fields
/// inlined on the policy) means accepting a later, different invitee on
/// the same policy id never requires resizing or migrating the policy
/// account itself.
#[account]
#[derive(InitSpace)]
pub struct VestingEnrollment {
  pub policy: Pubkey,
  pub invitee: Pubkey,
  pub claimed_amount: u64,
  /// Index of the next tranche not yet claimed.
  pub next_tranche_index: u8,
  /// Set at cancellation to the index of the first tranche that was
  /// swept back to `maker` (unvested at that moment) — permanently
  /// caps which indices can ever be claimed, independent of how much
  /// later a claim is attempted. Without this, a claim long after
  /// cancellation could re-request funds that were already swept away,
  /// since time alone would otherwise make a swept tranche look
  /// "unlocked" again. None = no cancellation has happened.
  pub claim_ceiling: Option<u8>,
  pub status: EnrollmentStatus,
  pub invited_at: i64,
  pub accepted_at: Option<i64>,
  pub bump: u8,
}

impl VestingEnrollment {
  pub const SEED_PREFIX: &'static [u8] = VESTING_ENROLLMENT_SEED;
  
  /// Tranches with index < this bound are ever eligible to claim —
  /// either every tranche (no cancellation yet) or only the ones that
  /// had already vested at the moment of cancellation.
  pub fn claimable_tranche_count(&self, total_tranches: usize) -> usize {
    self.claim_ceiling
      .map(|c| c as usize)
      .unwrap_or(total_tranches)
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq)]
pub enum EnrollmentStatus {
  /// Maker signed the invitation; invitee has not yet accepted.
  Invited,
  /// Invitee accepted; tranches can be claimed as they unlock.
  Active,
  /// Maker cancelled — see `claim_ceiling`. Whatever had already
  /// vested as of cancellation is still claimable.
  Cancelled,
  /// Every claimable tranche (up to `claim_ceiling`, if any) has been
  /// claimed. Terminal, whether reached via full natural vesting or
  /// via claiming out a cancellation's salvage.
  Completed,
}