use super::*;
use ink_model::ContractState;
use primitives::*;

pub trait Predicate<T, B, W, I, C, D>: ContractState
where
    T: Member + Codec,
    B: Member + Codec,
    W: Codec,
    I: Member + SimpleArithmetic + Codec,
    C: commitment::traits::Commitment,
    D: deposit::traits::Deposit<I, C>,
{
    /// deplpy predicate contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    );

    /// Predicates MUST define a custom _witness struct for their particular type of state.
    /// Predicates MUST disallow state transitions which pass verification without some interested party’s consent, e.g. the owner’s signature
    fn verify_transaction(
        &self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        pre_state: &StateUpdate<T, I>,
        transaction: &Transaction<B, I>,
        witness: &W,
        post_state: &StateUpdate<T, I>,
    ) -> bool;

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T, I>,
    ) -> Result<ExitStarted>;

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    /// *(Common Part of DepreacteExit.)
    fn _deprecate_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        deprecated_exit: &Checkpoint<T, I>,
        transaction: &Transaction<B, I>,
        witness: &W,
        post_state: &StateUpdate<T, I>,
    ) -> primitives::Result<()> {
        if deprecated_exit.state_update.state_object.predicate != transaction.predicate {
            return Err("Transactions can only act on SUs with the same predicate contract.");
        }
        if post_state.state_object.predicate != transaction.predicate {
            return Err("Transactions can only produce SUs with the same deposit contract.");
        }
        if !primitives::is_intersects(&deprecated_exit.sub_range, &post_state.range) {
            return Err(
                "Transactions can only deprecate an exit intersecting the postState subrange.",
            );
        }
        if !self.verify_transaction(
            env,
            &deprecated_exit.state_update,
            transaction,
            witness,
            post_state,
        ) {
            return Err("Predicate must be able to verify the transaction to deprecate.");
        }
		Ok(())
    }

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    fn deprecate_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        deprecated_exit: Checkpoint<T, I>,
        transaction: Transaction<B, I>,
        witness: W,
        post_state: StateUpdate<T, I>,
    ) -> Result<()>;

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<T, I>,
        deposited_range_id: I,
    ) -> Result<ExitFinalized<T>>;

    fn commitment(&self) -> &C;
    fn deposit(&self) -> &D;
}
