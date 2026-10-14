use anchor_lang::prelude::*;

use crate::error::ErrorCode;
use crate::events::ApiHolderActivated;
use crate::state::ApiHolder;

pub fn handler(ctx: Context<ActivateApiHolder>) -> Result<()> {
  let api_holder = &mut ctx.accounts.api_holder;
  api_holder.require_owner(&ctx.accounts.owner.key())?;
  require!(!api_holder.is_active, ErrorCode::AlreadyActive);
  
  api_holder.is_active = true;
  
  emit_cpi!(ApiHolderActivated {
        api_holder: api_holder.key(),
    });
  
  Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct ActivateApiHolder<'info> {
  pub owner: Signer<'info>,
  
  #[account(
        mut,
        seeds = [ApiHolder::SEED_PREFIX, owner.key().as_ref()],
        bump = api_holder.bump,
  )]
  pub api_holder: Account<'info, ApiHolder>,
}