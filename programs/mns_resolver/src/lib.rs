use anchor_lang::prelude::*;

declare_id!("MNSR111111111111111111111111111111111111111");

#[program]
pub mod mns_resolver {
    use super::*;

    pub fn initialize_resolver(
        ctx: Context<InitializeResolver>,
        name: String,
    ) -> Result<()> {
        let resolver = &mut ctx.accounts.resolver;
        resolver.name = name;
        resolver.owner = ctx.accounts.owner.key();
        resolver.bump = ctx.bumps.resolver;
        
        Ok(())
    }

    pub fn set_address(
        ctx: Context<UpdateResolver>,
        _name: String,
        chain_id: u16,
        address: [u8; 32],
    ) -> Result<()> {
        let resolver = &mut ctx.accounts.resolver;
        
        // Find or create address record
        if let Some(record) = resolver.addresses.iter_mut().find(|r| r.chain_id == chain_id) {
            record.address = address;
        } else {
            require!(resolver.addresses.len() < 10, ResolverError::TooManyAddresses);
            resolver.addresses.push(AddressRecord {
                chain_id,
                address,
            });
        }
        
        emit!(AddressUpdated {
            name: resolver.name.clone(),
            chain_id,
            address,
        });
        
        Ok(())
    }

    pub fn set_text_record(
        ctx: Context<UpdateResolver>,
        _name: String,
        key: String,
        value: String,
    ) -> Result<()> {
        require!(key.len() <= 32, ResolverError::KeyTooLong);
        require!(value.len() <= 256, ResolverError::ValueTooLong);
        
        let resolver = &mut ctx.accounts.resolver;
        
        if let Some(record) = resolver.text_records.iter_mut().find(|r| r.key == key) {
            record.value = value.clone();
        } else {
            require!(resolver.text_records.len() < 20, ResolverError::TooManyTextRecords);
            resolver.text_records.push(TextRecord {
                key: key.clone(),
                value: value.clone(),
            });
        }
        
        emit!(TextRecordUpdated {
            name: resolver.name.clone(),
            key,
            value,
        });
        
        Ok(())
    }

    pub fn set_content_hash(
        ctx: Context<UpdateResolver>,
        _name: String,
        content_hash: [u8; 32],
    ) -> Result<()> {
        let resolver = &mut ctx.accounts.resolver;
        resolver.content_hash = Some(content_hash);
        
        emit!(ContentHashUpdated {
            name: resolver.name.clone(),
            content_hash,
        });
        
        Ok(())
    }

    pub fn set_moltbook_agent(
        ctx: Context<UpdateResolver>,
        _name: String,
        agent_id: String,
    ) -> Result<()> {
        require!(agent_id.len() <= 64, ResolverError::AgentIdTooLong);
        
        let resolver = &mut ctx.accounts.resolver;
        resolver.moltbook_agent_id = Some(agent_id.clone());
        
        emit!(MoltbookAgentUpdated {
            name: resolver.name.clone(),
            agent_id,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct InitializeResolver<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Resolver::INIT_SPACE,
        seeds = [b"resolver", name.as_bytes()],
        bump
    )]
    pub resolver: Account<'info, Resolver>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct UpdateResolver<'info> {
    #[account(
        mut,
        seeds = [b"resolver", name.as_bytes()],
        bump = resolver.bump,
        constraint = resolver.owner == owner.key() @ ResolverError::NotOwner
    )]
    pub resolver: Account<'info, Resolver>,
    
    pub owner: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Resolver {
    #[max_len(12)]
    pub name: String,
    pub owner: Pubkey,
    #[max_len(10)]
    pub addresses: Vec<AddressRecord>,
    #[max_len(20)]
    pub text_records: Vec<TextRecord>,
    pub content_hash: Option<[u8; 32]>,
    #[max_len(64)]
    pub moltbook_agent_id: Option<String>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct AddressRecord {
    pub chain_id: u16,
    pub address: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct TextRecord {
    #[max_len(32)]
    pub key: String,
    #[max_len(256)]
    pub value: String,
}

#[error_code]
pub enum ResolverError {
    #[msg("Not the owner of this resolver")]
    NotOwner,
    #[msg("Too many address records (max 10)")]
    TooManyAddresses,
    #[msg("Too many text records (max 20)")]
    TooManyTextRecords,
    #[msg("Key too long (max 32 characters)")]
    KeyTooLong,
    #[msg("Value too long (max 256 characters)")]
    ValueTooLong,
    #[msg("Agent ID too long (max 64 characters)")]
    AgentIdTooLong,
}

#[event]
pub struct AddressUpdated {
    pub name: String,
    pub chain_id: u16,
    pub address: [u8; 32],
}

#[event]
pub struct TextRecordUpdated {
    pub name: String,
    pub key: String,
    pub value: String,
}

#[event]
pub struct ContentHashUpdated {
    pub name: String,
    pub content_hash: [u8; 32],
}

#[event]
pub struct MoltbookAgentUpdated {
    pub name: String,
    pub agent_id: String,
}


