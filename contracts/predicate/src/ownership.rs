use super::*;
use commitment::traits::Verify;
use core::marker::PhantomData;
use deposit::traits::Deposit;
use ink_core::{
    memory::{format, vec::Vec},
    storage,
};
use primitives::default::*;

ink_model::state! {
    pub struct Predicate {
        // deposit contract
        DEPOSIT: deposit::default::Deposit,
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct TransactionBody {
    new_state: StateObject<AccountId>,
    origin_block: BlockNumber,
    max_block: BlockNumber,
}

#[derive(Clone, Encode, Decode)]
pub struct Signature(pub [u8; 64]);
pub fn check_signature<T: Codec>(data: &T, pubkey: &AccountId, signature: &Signature) -> bool {
    // TODO check signature, but now can not ink! signature logic.
    // TODO Waiting efficient built-in cryptographic functions. (https://github.com/paritytech/ink/issues/6)
    true
}

impl
    traits::Predicate<
        AccountId,
        TransactionBody,
        Signature,
        RangeNumber,
        commitment::default::Commitment,
        deposit::default::Deposit,
    > for Predicate
{
    /// Deplpy predicate contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    ) {
        self.deposit()
            .deploy(env, token_address, chalenge_period, exit_period);
    }

    /// The main thing that must be defined for a state transition model is this verifyTransaction function
    /// which accepts a preState state update, and verifies against a transaction and witness that a given postState is correct.
    fn verify_transaction(
        &self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        pre_state: &StateUpdate<AccountId>,
        transaction: &Transaction<TransactionBody>,
        witness: &Signature,
        post_state: &StateUpdate<AccountId>,
    ) -> bool {
        /// Define a custom witness struct for their particular type of state.
        let owner = &pre_state.state_object.data;
        if !check_signature(transaction, owner, witness) {
            env.println("Owner must have signed the transaction.");
            return false;
        }

        // Disallow state transitions which pass verification without some interested party’s consent, e.g. the owner’s signature
        // check the prestate came after or at the originating block
        if pre_state.plasma_block_number > transaction.body.origin_block {
            env.println(
                "Transaction preState must come before or on the transaction body origin block.",
            );
            return false;
        }
        // check the poststate came before or at the max block
        if post_state.plasma_block_number > transaction.body.max_block {
            env.println(
                "Transaction postState must come before or on the transaction body max block.",
            );
            return false;
        }
        // check the state objects are the same
        if post_state.state_object != transaction.body.new_state {
            env.println("postState must be the transaction.body.newState.");
            return false;
        }
        true
    }

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<AccountId>,
    ) -> primitives::Result<ExitStarted> {
        // Extract the owner from the state object data field
        let owner = &checkpoint.state_update.state_object.data;

        // Require that this is called by the owner
        if &env.caller() != owner {
            return Err("Only owner may initiate the exit.");
        }

        // Forward the authenticated startExit to the deposit contract
        self.deposit().start_exit(env, checkpoint)
    }

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<AccountId>,
        deposited_range_id: RangeNumber,
    ) -> primitives::Result<ExitFinalized<AccountId>> {
        // Extract the owner from the state object data field
        let owner = &exit.state_update.state_object.data;
        // Require that this is called by the owner
        if &env.caller() != owner {
            return Err("Only owner may finalize the exit.");
        }
        // handle the finalization from the parent class now thaat we've verified it's authenticated
        self.deposit().finalize_exit(env, exit, deposited_range_id)
    }

    fn commitment(&mut self) -> &mut commitment::default::Commitment {
        self.deposit().commitment()
    }
    fn deposit(&mut self) -> &mut deposit::default::Deposit {
        &mut self.DEPOSIT
    }
}
