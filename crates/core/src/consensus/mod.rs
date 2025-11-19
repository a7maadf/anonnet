/// Consensus and credit system
///
/// This module implements the credit-based economy and consensus mechanism for AnonNet.
/// Nodes earn credits by relaying traffic and spend credits to use the network.

mod block;
mod ledger;
mod transaction;
mod validator;

pub use block::{Block, BlockError, BlockHeader, Blockchain};
pub use ledger::{CreditLedger, TransactionValidator};
pub use transaction::{
    RelayProof, Transaction, TransactionError, TransactionId, TransactionType,
};
pub use validator::{Validator, ValidatorError, ValidatorSet};
