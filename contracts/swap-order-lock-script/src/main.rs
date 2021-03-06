//! Generated by capsule
//!
//! `main.rs` is used to define rust lang items and modules.
//! See `entry.rs` for the `main` function.
//! See `error.rs` for the `Error` type.

#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

mod error;

use core::result::Result;

use num_bigint::BigUint;
use share::cell::SwapRequestLockArgs;
use share::ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    default_alloc,
    high_level::{
        load_cell, load_cell_data, load_cell_lock_hash, load_cell_type, load_script,
        load_witness_args, QueryIter,
    },
};
use share::{ckb_std, decode_u128, get_cell_type_hash};

use crate::error::Error;

// Alloc 4K fast HEAP + 2M HEAP to receives PrefilledData
default_alloc!(4 * 1024, 2048 * 1024, 64);

ckb_std::entry!(program_entry);

/// program entry
fn program_entry() -> i8 {
    // Call main function and return error code
    match main() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}

fn main() -> Result<(), Error> {
    let script = load_script()?;
    let lock_args = SwapRequestLockArgs::from_raw(script.args().as_slice())?;

    for (idx, lock_hash) in QueryIter::new(load_cell_lock_hash, Source::Input).enumerate() {
        if lock_hash == lock_args.user_lock_hash {
            let witness = load_witness_args(idx, Source::Input)?;
            if witness.total_size() != 0 {
                return Ok(());
            }
        }
    }

    let mut index = 0;
    let self_cell = load_cell(0, Source::GroupInput)?;
    for (idx, cell) in QueryIter::new(load_cell, Source::Input).enumerate().skip(3) {
        if cell.as_slice() == self_cell.as_slice() {
            index = idx;
            break;
        }
    }

    let order_cell = load_cell(index, Source::Input)?;
    let output_cell = load_cell(index, Source::Output)?;
    let order_lock_args = SwapRequestLockArgs::from_raw(&load_cell_data(index, Source::Input)?)?;

    if load_cell_lock_hash(index, Source::Output)? != order_lock_args.user_lock_hash {
        return Err(Error::InvalidOutputLockHash);
    }

    // if order_lock_args.kind == OrderKind::SellCKB {
    if load_cell_type(index, Source::Input)?.is_none() {
        // Ckb -> SUDT
        if order_lock_args.sudt_type_hash == get_cell_type_hash!(index, Source::Output) {
            return Err(Error::InvalidOutputTypeHash);
        }

        if order_cell.capacity().unpack() <= output_cell.capacity().unpack() {
            return Err(Error::InvalidCapacity);
        }

        if decode_u128(&load_cell_data(index, Source::Output)?)? < order_lock_args.min_amount_out {
            return Err(Error::SwapAmountLessThanMin);
        }
    } else {
        // SUDT -> Ckb
        if output_cell.type_().is_some() {
            return Err(Error::InvalidOutputTypeHash);
        }

        if BigUint::from(output_cell.capacity().unpack())
            < BigUint::from(order_cell.capacity().unpack()) + order_lock_args.min_amount_out
        {
            return Err(Error::InvalidCapacity);
        }

        if !load_cell_data(index, Source::Output)?.is_empty() {
            return Err(Error::InvalidOutputData);
        }
    }

    Ok(())
}
