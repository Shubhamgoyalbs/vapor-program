use anchor_lang::prelude::*;

/// program fee receiver
pub const TREASURY_ADMIN: Pubkey = pubkey!("4ar2w6ovLfx6GgCTchmKnPbwnYYhFFCNcdcTJ5EcPabZ");

/// Percentage precision used everywhere a percentage/fee/split is stored
/// on-chain (fees, split shares, refund portions, vesting tranches) —
/// matches your reference file's scheme.
/// Formula: `value / PERCENTAGE_PRECISION * 100 = %`
/// Examples: 1_000_000 = 1% | 50_000_000 = 50% | 100_000_000 = 100%
pub const PERCENTAGE_PRECISION: u32 = 100_000_000;

/// Program-level fee taken on every settled payment, subscription charge,
/// and vesting claim, in PERCENTAGE_PRECISION units. Placeholder — swap
/// for your real number before deploying. (250_000 = 0.25%)
pub const PROGRAM_FEE_PERCENTAGE: u32 = 250_000;

/// Every api_holder's vault must retain at least this many lamports for
/// the account to be considered active. Global and fixed, not a
/// per-holder field — it's a rent/liveness floor, not a business term
/// that should vary holder-to-holder.
/// Placeholder — size to your actual rent-exemption costs.
pub const MIN_VAULT_LAMPORTS: u64 = 10_000_000; // 0.01 SOL

/// One-time gas top-up sent to `auth` (the Privy-managed wallet) at
/// `init_api_holder`, so it can immediately pay for its own transaction
/// fees when it starts submitting instructions on the holder's behalf.
/// Placeholder — size to real fee costs for your instruction mix.
pub const AUTH_GAS_LAMPORTS: u64 = 5_000_000; // 0.005 SOL

/// Max byte length accepted for any client-supplied order/policy id
/// (CUID2 or UUID). Hard ceiling, not a style choice: Solana caps each
/// individual PDA seed at 32 bytes.
///   - CUID2 default output (24 chars) fits with room to spare.
///   - UUID MUST be passed with hyphens stripped (32 hex chars = 32
///     bytes, fits exactly). The hyphenated 36-char form will not fit.
/// NOTE: your reference file uses `#[max_len(25)]` for its cuid fields —
/// fine for CUID2-only, but too small for a stripped UUID (32 bytes).
/// Keeping 32 here since the spec explicitly asks for both CUID and
/// UUID support; shrink to 25 only if you're dropping UUID.
pub const MAX_ID_LEN: usize = 32;

/// Sentinel stored in `PaymentOrder.mint` for a native-SOL order (paid
/// via plain lamport transfers, not wrapped SOL / SPL at all) — the
/// System Program's own address, which can never be a real SPL mint.
/// `create_payment_order_sol` / `pay_order_sol` / `settle_order_sol` /
/// `refund_order_sol` are the matching native-SOL instruction set.
pub const NATIVE_SOL_MINT_SENTINEL: Pubkey = anchor_lang::system_program::ID;

/// Max split-payment recipients on a single payment order (fixed-size
/// array, matching your reference's `[Option<SplitShare>; 7]` pattern).
pub const MAX_SPLIT_RECIPIENTS: usize = 7;

/// Max addresses in a payment order's payer allowlist. Empty = open to
/// anyone (bounded only by `payer_cap`, if set).
pub const MAX_PAYER_ALLOWLIST: usize = 16;

/// Max tranches on a single vesting policy.
pub const MAX_VESTING_TRANCHES: usize = 16;

/// Time period (in seconds) after which an unaccepted vesting enrollment
/// invite can be swept back to the maker. If an invitee hasn't accepted
/// within this window, the maker can reclaim the funds.
pub const INVITE_EXPIRY_SECONDS: i64 = 30 * 24 * 60 * 60; // 30 days