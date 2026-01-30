use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;

declare_id!("MNS1111111111111111111111111111111111111111");

#[program]
pub mod mns_registry {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.authority = ctx.accounts.authority.key();
        registry.total_registered = 0;
        registry.fee_lamports = 100_000_000; // 0.1 SOL
        registry.bump = ctx.bumps.registry;
        
        emit!(RegistryInitialized {
            authority: registry.authority,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    pub fn register_name(
        ctx: Context<RegisterName>,
        name: String,
    ) -> Result<()> {
        require!(name.len() >= 3 && name.len() <= 12, MnsError::InvalidNameLength);
        require!(is_valid_name(&name), MnsError::InvalidNameCharacters);
        
        let name_record = &mut ctx.accounts.name_record;
        let registry = &mut ctx.accounts.registry;
        
        name_record.name = name.clone();
        name_record.owner = ctx.accounts.owner.key();
        name_record.resolver = Pubkey::default();
        name_record.registered_at = Clock::get()?.unix_timestamp;
        name_record.expires_at = Clock::get()?.unix_timestamp + 31_536_000; // 1 year
        name_record.bump = ctx.bumps.name_record;
        
        registry.total_registered = registry.total_registered.checked_add(1).unwrap();
        
        // Transfer registration fee
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, registry.fee_lamports)?;
        
        emit!(NameRegistered {
            name: name.clone(),
            owner: ctx.accounts.owner.key(),
            expires_at: name_record.expires_at,
        });
        
        Ok(())
    }

    pub fn transfer_name(
        ctx: Context<TransferName>,
        name: String,
    ) -> Result<()> {
        let name_record = &mut ctx.accounts.name_record;
        let new_owner = ctx.accounts.new_owner.key();
        let previous_owner = name_record.owner;
        
        name_record.owner = new_owner;
        
        emit!(NameTransferred {
            name,
            from: previous_owner,
            to: new_owner,
        });
        
        Ok(())
    }

    pub fn set_resolver(
        ctx: Context<SetResolver>,
        _name: String,
        resolver: Pubkey,
    ) -> Result<()> {
        let name_record = &mut ctx.accounts.name_record;
        name_record.resolver = resolver;
        
        emit!(ResolverUpdated {
            name: name_record.name.clone(),
            resolver,
        });
        
        Ok(())
    }

    pub fn renew_name(
        ctx: Context<RenewName>,
        _name: String,
        years: u8,
    ) -> Result<()> {
        require!(years > 0 && years <= 5, MnsError::InvalidRenewalPeriod);
        
        let name_record = &mut ctx.accounts.name_record;
        let registry = &ctx.accounts.registry;
        
        let extension = (years as i64) * 31_536_000;
        name_record.expires_at = name_record.expires_at.checked_add(extension).unwrap();
        
        let renewal_fee = registry.fee_lamports.checked_mul(years as u64).unwrap();
        
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, renewal_fee)?;
        
        emit!(NameRenewed {
            name: name_record.name.clone(),
            new_expiry: name_record.expires_at,
        });
        
        Ok(())
    }

    pub fn update_fee(
        ctx: Context<UpdateRegistry>,
        new_fee: u64,
    ) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.fee_lamports = new_fee;
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Registry::INIT_SPACE,
        seeds = [b"registry"],
        bump
    )]
    pub registry: Account<'info, Registry>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct RegisterName<'info> {
    #[account(
        mut,
        seeds = [b"registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, Registry>,
    
    #[account(
        init,
        payer = owner,
        space = 8 + NameRecord::INIT_SPACE,
        seeds = [b"name", name.as_bytes()],
        bump
    )]
    pub name_record: Account<'info, NameRecord>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: Treasury account for fee collection
    #[account(mut)]
    pub treasury: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct TransferName<'info> {
    #[account(
        mut,
        seeds = [b"name", name.as_bytes()],
        bump = name_record.bump,
        constraint = name_record.owner == owner.key() @ MnsError::NotOwner
    )]
    pub name_record: Account<'info, NameRecord>,
    
    pub owner: Signer<'info>,
    
    /// CHECK: New owner receiving the name
    pub new_owner: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct SetResolver<'info> {
    #[account(
        mut,
        seeds = [b"name", name.as_bytes()],
        bump = name_record.bump,
        constraint = name_record.owner == owner.key() @ MnsError::NotOwner
    )]
    pub name_record: Account<'info, NameRecord>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct RenewName<'info> {
    #[account(
        seeds = [b"registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, Registry>,
    
    #[account(
        mut,
        seeds = [b"name", name.as_bytes()],
        bump = name_record.bump,
        constraint = name_record.owner == owner.key() @ MnsError::NotOwner
    )]
    pub name_record: Account<'info, NameRecord>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: Treasury account
    #[account(mut)]
    pub treasury: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRegistry<'info> {
    #[account(
        mut,
        seeds = [b"registry"],
        bump = registry.bump,
        constraint = registry.authority == authority.key() @ MnsError::Unauthorized
    )]
    pub registry: Account<'info, Registry>,
    
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Registry {
    pub authority: Pubkey,
    pub total_registered: u64,
    pub fee_lamports: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct NameRecord {
    #[max_len(12)]
    pub name: String,
    pub owner: Pubkey,
    pub resolver: Pubkey,
    pub registered_at: i64,
    pub expires_at: i64,
    pub bump: u8,
}

#[error_code]
pub enum MnsError {
    #[msg("Name must be between 3 and 12 characters")]
    InvalidNameLength,
    #[msg("Name can only contain lowercase letters, numbers, and underscores")]
    InvalidNameCharacters,
    #[msg("Not the owner of this name")]
    NotOwner,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Renewal period must be between 1 and 5 years")]
    InvalidRenewalPeriod,
}

#[event]
pub struct RegistryInitialized {
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct NameRegistered {
    pub name: String,
    pub owner: Pubkey,
    pub expires_at: i64,
}

#[event]
pub struct NameTransferred {
    pub name: String,
    pub from: Pubkey,
    pub to: Pubkey,
}

#[event]
pub struct ResolverUpdated {
    pub name: String,
    pub resolver: Pubkey,
}

#[event]
pub struct NameRenewed {
    pub name: String,
    pub new_expiry: i64,
}

