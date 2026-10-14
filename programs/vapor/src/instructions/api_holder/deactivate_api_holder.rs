use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::ApiHolderDeactivated;
use crate::state::ApiHolder;

/// Deactivating only blocks `auth`-gated instructions (create_order,
/// create_subscription_policy, ...) via `ApiHolder::require_auth`.
/// It does NOT freeze `deposit` / `withdraw` — those check `owner`
/// directly and are unaffected by this flag.
pub fn handler(ctx: Context<DeactivateApiHolder>) -> Result<()> {
  let api_holder = &mut ctx.accounts.api_holder;
  api_holder.require_owner(&ctx.accounts.owner.key())?;
  require!(api_holder.is_active, ErrorCode::AlreadyInactive);
  
  api_holder.is_active = false;
  
  emit_cpi!(ApiHolderDeactivated {
        api_holder: api_holder.key(),
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct DeactivateApiHolder<'info> {
  pub owner: Signer<'info>,
  
  #[account(
        mut,
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
}