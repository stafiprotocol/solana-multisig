//! An example of a multisig to execute arbitrary Solana transactions.
//!
//! This program can be used to allow a multisig to govern anything a regular
//! Pubkey can govern. One can use the multisig as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a `Multisig` account, specifying two important
//! parameters:
//!
//! 1. Owners - the set of addresses that sign transactions for the multisig.
//! 2. Threshold - the number of signers required to execute a transaction.
//!
//! Once the `Multisig` account is created, one can create a `Transaction`
//! account, specifying the parameters for a normal solana transaction.
//!
//! To sign, owners should invoke the `approve` instruction.

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use std::convert::Into;

#[program]
pub mod multisig {
    use super::*;

    // Initializes a new multisig account with a set of owners and a threshold.
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
        nonce: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.nonce = nonce;
        multisig.owner_set_seqno = 0;
        Ok(())
    }

    // Creates a new transaction account
    // which must be one of the owners of the multisig.
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pids: Vec<Pubkey>,
        accs: Vec<Vec<TransactionAccount>>,
        datas: Vec<Vec<u8>>,
    ) -> Result<()> {
        if pids.len() != accs.len() || pids.len() != datas.len() {
            return Err(ErrorCode::ParamLength.into());
        }
        let _ = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.proposer.key)
            .ok_or(ErrorCode::InvalidOwner)?;

        let mut signers = Vec::new();
        signers.resize(ctx.accounts.multisig.owners.len(), false);

        let tx = &mut ctx.accounts.transaction;
        tx.program_ids = pids;
        tx.accounts = accs;
        tx.datas = datas;
        tx.signers = signers;
        tx.multisig = *ctx.accounts.multisig.to_account_info().key;
        tx.did_execute = false;
        tx.owner_set_seqno = ctx.accounts.multisig.owner_set_seqno;

        Ok(())
    }

    // Approve and Executes the given transaction if threshold owners have signed it.
    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(ErrorCode::InvalidOwner)?;

        ctx.accounts.transaction.signers[owner_index] = true;

        // Do we have enough signers.
        let sig_count = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter(|&did_sign| *did_sign)
            .count() as u64;
        if sig_count < ctx.accounts.multisig.threshold {
            return Ok(());
        }

        // Has this been executed already?
        if ctx.accounts.transaction.did_execute {
            return Err(ErrorCode::AlreadyExecuted.into());
        }

        // Execute the transaction signed by the multisig.
        let mut ixs: Vec<Instruction> = (&*ctx.accounts.transaction).into();
        for ix in ixs.iter_mut() {
            ix.accounts = ix
                .accounts
                .iter()
                .map(|acc| {
                    let mut acc = acc.clone();
                    if &acc.pubkey == ctx.accounts.multisig_signer.key {
                        acc.is_signer = true;
                    }
                    acc
                })
                .collect();
        }

        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),
            &[ctx.accounts.multisig.nonce],
        ];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        for ix in ixs.iter() {
            solana_program::program::invoke_signed(ix, &accounts, signer)?;
        }

        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        Ok(())
    }
}

// Sets the owners field on the multisig. The only way this can be invoked
// is via a recursive call from execute_transaction -> set_owners.
pub fn set_owners(ctx: Context<Auth>, owners: Vec<Pubkey>) -> Result<()> {
    let owners_len = owners.len() as u64;
    if owners_len == 0 {
        return Err(ErrorCode::InvalidOwnerLength.into());
    }

    let multisig = &mut ctx.accounts.multisig;
    if owners_len < multisig.threshold {
        multisig.threshold = owners_len;
    }

    multisig.owners = owners;
    multisig.owner_set_seqno += 1;
    Ok(())
}

// Changes the execution threshold of the multisig. The only way this can be
// invoked is via a recursive call from execute_transaction ->
// change_threshold.
pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {
    if threshold == 0 {
        return Err(ErrorCode::InvalidThreshold.into());
    }
    if threshold > ctx.accounts.multisig.owners.len() as u64 {
        return Err(ErrorCode::InvalidThreshold.into());
    }
    let multisig = &mut ctx.accounts.multisig;
    multisig.threshold = threshold;
    Ok(())
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    multisig: ProgramAccount<'info, Multisig>,
    #[account(signer, seeds = [
        multisig.to_account_info().key.as_ref(),
        &[multisig.nonce],
    ])]
    multisig_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    #[account(init)]
    multisig: ProgramAccount<'info, Multisig>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(init)]
    transaction: ProgramAccount<'info, Transaction>,
    // One of the owners. Checked in the handler.
    #[account(signer)]
    proposer: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    multisig: ProgramAccount<'info, Multisig>,
    #[account(seeds = [
        multisig.to_account_info().key.as_ref(),
        &[multisig.nonce],
    ])]
    multisig_signer: AccountInfo<'info>,
    #[account(mut, has_one = multisig)]
    transaction: ProgramAccount<'info, Transaction>,
    // One of the multisig owners. Checked in the handler.
    #[account(signer)]
    owner: AccountInfo<'info>,
}

#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub nonce: u8,
    pub owner_set_seqno: u32,
}

#[account]
pub struct Transaction {
    // The multisig account this transaction belongs to.
    pub multisig: Pubkey,
    // Target program to execute against.
    pub program_ids: Vec<Pubkey>,
    // Accounts requried for the transaction.
    pub accounts: Vec<Vec<TransactionAccount>>,
    // Instruction datas for the transaction.
    pub datas: Vec<Vec<u8>>,
    // signers[index] is true if multisig.owners[index] signed the transaction.
    pub signers: Vec<bool>,
    // Boolean ensuring one time execution.
    pub did_execute: bool,
    // Owner set sequence number.
    pub owner_set_seqno: u32,
}

impl From<&Transaction> for Vec<Instruction> {
    fn from(tx: &Transaction) -> Vec<Instruction> {
        let mut instructions: Vec<Instruction> = Vec::new();
        for (i, _pid) in tx.program_ids.iter().enumerate() {
            instructions.push(Instruction {
                program_id: tx.program_ids[i],
                accounts: tx.accounts[i].iter().map(AccountMeta::from).collect(),
                data: tx.datas[i].clone(),
            })
        }
        instructions
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[error]
pub enum ErrorCode {
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
    #[msg("The given owners is empty.")]
    InvalidOwnerLength,
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    #[msg("Cannot delete a transaction that has been signed by an owner.")]
    TransactionAlreadySigned,
    #[msg("Overflow when adding.")]
    Overflow,
    #[msg("Cannot delete a transaction the owner did not create.")]
    UnableToDelete,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    #[msg("program id account data must have same length")]
    ParamLength,
}
