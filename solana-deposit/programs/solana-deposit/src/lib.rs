use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer as SplTransfer};

declare_id!("ErRaZ4rnCLC3nZdwwHuUTtgqnDD3UFGCsBDvsxii1X3i");

pub const CONFIG_SEED: &[u8] = b"config";
pub const VAULT_SEED: &[u8] = b"vault";

#[program]
pub mod solana_deposit {
    use super::*;

    /// Initialize config PDA, SOL vault PDA and the vault's USDC ATA.
    /// The signer becomes the program admin (the only one allowed to withdraw).
    pub fn initialize(
        ctx: Context<Initialize>,
        min_deposit_lamports: u64,
        min_deposit_usdc: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.usdc_mint = ctx.accounts.usdc_mint.key();
        config.min_deposit_lamports = min_deposit_lamports;
        config.min_deposit_usdc = min_deposit_usdc;
        config.bump = ctx.bumps.config;
        config.vault_bump = ctx.bumps.vault;
        Ok(())
    }

    /// Update min deposit thresholds. Admin only.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        min_deposit_lamports: u64,
        min_deposit_usdc: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.min_deposit_lamports = min_deposit_lamports;
        config.min_deposit_usdc = min_deposit_usdc;
        Ok(())
    }

    /// Deposit SOL into the vault. `user_id` is the sub2api user id and is
    /// carried by the emitted event so the off-chain watcher can credit the
    /// right account.
    pub fn deposit_sol(ctx: Context<DepositSol>, amount: u64, user_id: u64) -> Result<()> {
        require!(amount > 0, DepositError::ZeroAmount);
        require!(
            amount >= ctx.accounts.config.min_deposit_lamports,
            DepositError::BelowMinimum
        );

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(DepositEvent {
            user_id,
            token: DepositToken::Sol,
            amount,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    /// Deposit USDC (SPL) into the vault ATA.
    pub fn deposit_usdc(ctx: Context<DepositUsdc>, amount: u64, user_id: u64) -> Result<()> {
        require!(amount > 0, DepositError::ZeroAmount);
        require!(
            amount >= ctx.accounts.config.min_deposit_usdc,
            DepositError::BelowMinimum
        );

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.payer_usdc_ata.to_account_info(),
                    to: ctx.accounts.vault_usdc_ata.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(DepositEvent {
            user_id,
            token: DepositToken::Usdc,
            amount,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    /// Withdraw SOL from the vault to `recipient`. Admin only.
    /// The vault account is owned by this program, so lamports are moved
    /// directly (no CPI signature needed).
    pub fn withdraw_sol(ctx: Context<WithdrawSol>, amount: u64) -> Result<()> {
        require!(amount > 0, DepositError::ZeroAmount);

        let vault_info = ctx.accounts.vault.to_account_info();
        let recipient_info = ctx.accounts.recipient.to_account_info();

        let rent = Rent::get()?;
        let rent_minimum = rent.minimum_balance(vault_info.data_len());
        let vault_lamports = vault_info.lamports();
        require!(
            vault_lamports >= amount && vault_lamports - amount >= rent_minimum,
            DepositError::InsufficientVaultBalance
        );

        **vault_info.try_borrow_mut_lamports()? -= amount;
        **recipient_info.try_borrow_mut_lamports()? += amount;

        emit!(WithdrawEvent {
            token: DepositToken::Sol,
            amount,
            recipient: ctx.accounts.recipient.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    /// Withdraw USDC from the vault ATA to `recipient_usdc_ata`. Admin only.
    pub fn withdraw_usdc(ctx: Context<WithdrawUsdc>, amount: u64) -> Result<()> {
        require!(amount > 0, DepositError::ZeroAmount);

        let vault_bump = ctx.accounts.config.vault_bump;
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, &[vault_bump]]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.vault_usdc_ata.to_account_info(),
                    to: ctx.accounts.recipient_usdc_ata.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        emit!(WithdrawEvent {
            token: DepositToken::Usdc,
            amount,
            recipient: ctx.accounts.recipient_usdc_ata.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,

    /// Vault PDA that custodies deposited SOL. Owned by this program,
    /// holds no data beyond the discriminator.
    #[account(
        init,
        payer = admin,
        space = 8,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub usdc_mint: Account<'info, Mint>,

    /// Vault PDA's USDC associated token account.
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ DepositError::Unauthorized
    )]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct DepositSol<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [VAULT_SEED], bump = config.vault_bump)]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositUsdc<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(seeds = [VAULT_SEED], bump = config.vault_bump)]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(constraint = usdc_mint.key() == config.usdc_mint @ DepositError::InvalidMint)]
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = payer
    )]
    pub payer_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawSol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ DepositError::Unauthorized
    )]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [VAULT_SEED], bump = config.vault_bump)]
    pub vault: Account<'info, Vault>,

    /// CHECK: any account designated by the admin to receive the SOL.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct WithdrawUsdc<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ DepositError::Unauthorized
    )]
    pub config: Account<'info, Config>,

    #[account(seeds = [VAULT_SEED], bump = config.vault_bump)]
    pub vault: Account<'info, Vault>,

    #[account(constraint = usdc_mint.key() == config.usdc_mint @ DepositError::InvalidMint)]
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_usdc_ata.mint == usdc_mint.key() @ DepositError::InvalidMint
    )]
    pub recipient_usdc_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub min_deposit_lamports: u64,
    pub min_deposit_usdc: u64,
    pub bump: u8,
    pub vault_bump: u8,
}

/// Marker account for the SOL vault PDA (no data besides discriminator).
#[account]
pub struct Vault {}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum DepositToken {
    Sol,
    Usdc,
}

#[event]
pub struct DepositEvent {
    pub user_id: u64,
    pub token: DepositToken,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub token: DepositToken,
    pub amount: u64,
    pub recipient: Pubkey,
    pub timestamp: i64,
}

#[error_code]
pub enum DepositError {
    #[msg("Deposit amount must be greater than zero")]
    ZeroAmount,
    #[msg("Deposit amount is below the configured minimum")]
    BelowMinimum,
    #[msg("Caller is not the program admin")]
    Unauthorized,
    #[msg("Token mint does not match the configured USDC mint")]
    InvalidMint,
    #[msg("Vault balance is insufficient for this withdrawal")]
    InsufficientVaultBalance,
}
