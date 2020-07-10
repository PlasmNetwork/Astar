//! # Ovm Module
//!
//! The Ovm module provides functionality for handling layer2 dispute logics.
//! This refer to: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/UniversalAdjudicationContract.sol
//!
//! - [`ovm::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//! Ovm module is the substrate pallet to archive dispute game defined by predicate logic.
//!
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Get, Time},
    weights::{DispatchClass, FunctionOf, Pays, Weight},
    Parameter, StorageMap,
};
use frame_system::{self as system, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{Hash, Member},
    RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod predicate;
pub mod traits;

use predicate::{ExecResult, ExecutionContext, PredicateLoader, PredicateOvm};
use traits::{Ext, PredicateAddressFor};

/// PredicateContract wrapped Predicate and initial arguments.
///
/// Required functions of each PredicateContract:
/// - isValidChallenge
/// - decide
///
/// isValidChallenge validates valid child node of game tree.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct PredicateContract<CodeHash> {
    pub predicate_hash: CodeHash,
    pub inputs: Vec<u8>,
}

/// Property stands for dispute logic and we can claim every Properties to Adjudicator Contract.
/// Property has its predicate address and array of input.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Property<AccountId> {
    /// Indicates the address of Predicate.
    pub predicate_address: AccountId,
    /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    pub inputs: Vec<Vec<u8>>,
}

/// The game decision by predicates.
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub enum Decision {
    Undecided,
    True,
    False,
}

/// ChallengeGame is a part of L2 dispute. It's instantiated by claiming property.
/// The client can get a game instance from this module.
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct ChallengeGame<AccountId, Hash, BlockNumber> {
    /// Property of challenging targets.
    property_hash: Hash,
    /// challenges inputs
    challenges: Vec<Hash>,
    /// the result of this challenge.
    decision: Decision,
    /// the block number when this was issued.
    created_block: BlockNumber,
}

/// Definition of the cost schedule and other parameterizations for optimistic virtual machine.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct Schedule {
    /// Version of the schedule.
    pub version: u32,

    /// Cost of putting a byte of code into storage.
    pub put_code_per_byte_cost: Weight,
}

// 500 (2 instructions per nano second on 2GHZ) * 1000x slowdown through wasmi
// This is a wild guess and should be viewed as a rough estimation.
// Proper benchmarks are needed before this value and its derivatives can be used in production.
const OVM_INSTRUCTION_COST: Weight = 500_000;

impl Default for Schedule {
    fn default() -> Schedule {
        Schedule {
            version: 0,
            put_code_per_byte_cost: OVM_INSTRUCTION_COST,
        }
    }
}

/// In-memory cache of configuration values.
///
/// We assume that these values can't be changed in the
/// course of transaction execution.
pub struct Config {
    pub schedule: Schedule,
    pub max_depth: u32, // about down 30.
}

impl Config {
    fn preload<T: Trait>() -> Config {
        Config {
            schedule: <Module<T>>::current_schedule(),
            max_depth: T::MaxDepth::get(),
        }
    }
}

type PredicateHash<T> = <T as system::Trait>::Hash;
type ChallengeGameOf<T> = ChallengeGame<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as system::Trait>::BlockNumber,
>;
pub type PropertyOf<T> = Property<<T as system::Trait>::AccountId>;
type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;
type PredicateContractOf<T> = PredicateContract<<T as frame_system::Trait>::Hash>;

pub trait Trait: system::Trait {
    /// The maximum nesting level of a call/instantiate stack.
    type MaxDepth: Get<u32>;

    /// During the dispute period defined here, the user can challenge.
    /// If nothing is found, the state is determined after the dispute period.
    type DisputePeriod: Get<Self::BlockNumber>;

    /// A function type to get the contract address given the instantiator.
    type DeterminePredicateAddress: PredicateAddressFor<PredicateHash<Self>, Self::AccountId>;

    /// The user account on Layer2 identifier type for the runtime.
    type AccountIdL2: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + Ord
        + Default;

    /// The hashing system (algorithm) being used in the runtime (e.g. Keccak256).
    type HashingL2: Hash<Output = Self::Hash>;

    /// ExternalCallError type.
    type ExternalCallError: From<&'static str>;

    /// ExternalCall context.
    type ExternalCall: Ext<Self, ovmi::executor::ExecError<Self::AccountIdL2>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Ovm {
        /// Current cost schedule for contracts.
        pub CurrentSchedule get(fn current_schedule) config(): Schedule = Schedule::default();

        /// A mapping from an original code hash to the original code, untouched by instrumentation.
        pub PredicateCodes get(fn predicate_codes): map hasher(identity) PredicateHash<T> => Option<Vec<u8>>;

        /// A mapping between an original code hash and instrumented ovm(predicate) code, ready for execution.
        pub PredicateCache get(fn predicate_cache): map hasher(identity) PredicateHash<T> => Option<predicate::PrefabOvmModule>;

        /// Mapping the predicate address to Predicate.
        /// Predicate is handled similar to contracts.
        pub Predicates get(fn predicates): map hasher(blake2_128_concat)
         T::AccountId => Option<PredicateContractOf<T>>;

        /// Mapping the game id to Challenge Game.
        pub Games get(fn games): map hasher(blake2_128_concat) T::Hash => Option<ChallengeGameOf<T>>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Property = PropertyOf<T>,
        Hash = <T as system::Trait>::Hash,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        /// (predicate_address: AccountId);
        PutPredicate(Hash),
        /// (predicate_address: AccountId);
        InstantiatePredicate(AccountId),
        /// (gameId: Hash, decision: bool)
        AtomicPropositionDecided(Hash, bool),
        /// (game_id: Hash, property: Property, createdBlock: BlockNumber)
        NewPropertyClaimed(Hash, Property, BlockNumber),
        /// (game_id: Hash, challengeGameId: Hash)
        ClaimChallenged(Hash, Hash),
        /// (game_id: Hash, decision: bool)
        ClaimDecided(Hash, bool),
        /// (game_id: Hash, property: Property, created_block: BlockNumber)
        PropertyClaimed(Hash, Property, BlockNumber),
        /// (gameId: Hash, challenge_game_id: Hash)
        PropertyChallenged(Hash, Hash),
        /// (game_id: Hash, decision: bool)
        PropertyDecided(Hash, bool),
        /// (game_id: Hash, challengeGameId: Hash)
        ChallengeRemoved(Hash, Hash),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Duplicate index.
        DuplicateIndex,
        /// claim isn't empty
        CiamIsNotEmpty,
        /// Does not exist game
        DoesNotExistGame,
        /// Does not exist predicate
        DoesNotExistPredicate,
        /// claim should be decidable
        ClaimShouldBeDecidable,
        /// challenge isn't valid
        ChallengeIsNotValid,
        /// challenging game haven't been decided true
        ChallengingGameNotTrue,
        /// Decision must be undecided
        DecisionMustBeUndecided,
        /// There must be no challenge
        ThereMustBeNoChallenge,
        /// property must be true with given witness
        PropertyMustBeTrue,
        /// challenging game haven't been decided false
        ChallengingGameNotFalse,
        /// setPredicateDecision must be called from predicate
        MustBeCalledFromPredicate,
        /// index must be less than challenges.length
        OutOfRangeOfChallenges,
        /// game is already started
        GameIsAlradyStarted,
        /// property is not claimed
        PropertyIsNotClaimed,
        /// challenge is already started
        ChallengeIsAlreadyStarted,
        /// challenge is not in the challenge list
        ChallengeIsNotInTheChallengeList,
        /// challenge property is not decided to false
        ChallengePropertyIsNotDecidedToFalse,
        /// challenge list is not empty
        ChallengeListIsNotEmpty,
        /// dispute period has not been passed
        DisputePeriodHasNotBeenPassed,
        /// undecided challenge exists
        UndecidedChallengeExists,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// During the dispute period defined here, the user can challenge.
        /// If nothing is found, the state is determined after the dispute period.
        const DisputePeriod: <T as system::Trait>::BlockNumber = T::DisputePeriod::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            // TODO: weight
            T::MaximumBlockWeight::get()
        }

        /// Stores the given binary Wasm code into the chain's storage and returns its `codehash`.
        /// You can instantiate contracts only with stored code.
        #[weight = FunctionOf(
            |args: (&Vec<u8>,)| Module::<T>::calc_code_put_costs(args.0),
            DispatchClass::Normal,
            Pays::Yes
        )]
        pub fn put_code(
            origin,
            predicate: Vec<u8>
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let schedule = Self::current_schedule();
            match predicate::save_code::<T>(predicate, &schedule) {
                Ok(predicate_hash) => {
                    Self::deposit_event(RawEvent::PutPredicate(predicate_hash));
                },
                Err(err) => return Err(err.into()),
            }
            Ok(())
        }

        /// Deploy predicate and made predicate address as AccountId.
        /// TODO: weight
        #[weight = 100_000]
        pub fn instantiate(origin, predicate_hash: PredicateHash<T>, inputs: Vec<u8>) {
            let origin = ensure_signed(origin)?;

            // Calc predicate address.
            let predicate_address = T::DeterminePredicateAddress::predicate_address_for(
                &predicate_hash,
                &inputs,
                &origin
            );
            let predicate_contract = PredicateContract {
                predicate_hash,
                inputs,
            };

            <Predicates<T>>::insert(&predicate_address, predicate_contract);

            Self::deposit_event(RawEvent::InstantiatePredicate(predicate_address));
        }

        /// Claims property and create new game. Id of game is hash of claimed property
        /// TODO: weight
        #[weight = 100_000]
        fn claim(origin, claim: PropertyOf<T>) {
            let origin = ensure_signed(origin)?;
            Self::only_from_dispute_contract(&origin, &claim)?;
            // get the id of this property
            let game_id = Self::get_property_id(&claim);
            ensure!(
                Self::started(game_id),
                Error::<T>::GameIsAlradyStarted,
            );

            let game = Self::create_game(game_id);
            <Games<T>>::insert(game_id, game);
           Self::deposit_event(RawEvent::PropertyClaimed(game_id, claim, block_number));
        }

        /// Challenge to an existing game instance by a property.
        ///
        /// challenge will be added to `challenges` field of challenged game instance.
        /// if property does not exist, revert.
        /// if challenge with same property was made before, revert.
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn challenge(origin, property: PropertyOf<T>, challenge_property: PropertyOf<T>) {
            let origin = ensure_signed(origin)?;
            Self::only_from_dispute_contract(&origin, &property)?;

            // validation
            let id = Self::get_property_id(&property);
            ensure!(
                Self::started(&id),
                Error::<T>::PropertyIsNotClaimed,
            );

            let challenge_game_id = Self::get_property_id(&property);
            ensure!(
                Self::started(&challenge_game_id),
                Error::<T>::ChallengeIsAlreadyStarted,
            );

            // start challenging game
            let challenge_game = Self::create_game(&challenge_game_id);
            <Games<T>>::insert(challenge_game_id, challenge_game);

            // add challenge to challenged game's challenge list
            <Games<T>::mutate(&id, |game| {
                *game.challenges.push(&challenge_game_id);
            });
            Self::deposit_event(RawEvent::PropertyChallenged(id, challenging_game_id));
        }

        /// remove challenge
        /// set challenging game decision to false and remove it from challenges field of challenged game
        /// if property does not exist, revert.
        /// if challenge property does not exist, revert.
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn remove_challenge(origin, property: PropertyOf<T>, challenge_property: PropertyOf<T>) {
            let origin = ensure_signed(origin)?;
            Self::only_from_dispute_contract(&origin, &property)?;

            let id = Self::get_property_id(&property);
            ensure!(
                Self::started(&id),
                Error::<T>::PropertyIsNotClaimed,
            );

            let challenge_game_id = Self::get_property_id(&property);
            ensure!(
                Self::started(&challenge_game_id),
                Error::<T>::ChallengeIsAlreadyStarted,
            );

            let mut game = Self::games(&id);
            let challenge_index = Self::find_index(&game.challenges, &challenge_game_id);
            ensure!(
                challenge_index >= 0,
                Error::<T>::ChallengeIsNotInTheChallengeList,
            );

            let challenge_game = Self::games(&challenge_game_id);
            ensure!(
                challenge_game.decision == Decision::False,
                Error::<T>::ChallengePropertyIsNotDecidedToFalse,
            );

            // remove challenge
            game.challenges = game
                .challenges
                .into_iter()
                .filter(|challenge| challenge != &challenging_game_id)
                .collect();
            <Games<T>>::insert(&game_id, game);
            Self::deposit_event(RawEvent::ChallengeRemoved(id, challenging_game_id));
        }

        /// set game result to given result value.
        /// only called from dispute contract
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn set_game_result(property: PropertyOf<T>, result: bool)
        {
            let origin = ensure_signed(origin)?;
            Self::only_from_dispute_contract(&origin, &property)?;

            let id = Self::get_property_id(&property);
            ensure!(
                Self::started(&id),
                Error::<T>::PropertyIsNotClaimed,
            );

            let mut game = Self::games(&id);
            ensure!(
                game.challenges.len() == 0,
                Error::<T>::ChallengeListIsNotEmpty,
            );

            game.decision = Self::get_decision(result);
            Self::deposit_event(RawEvent::PropertyDecided(id, result));
        }

        /// settle game
        /// settle started game whose dispute period has passed.
        /// if no challenge for the property exists, decide to true.
        /// if any of its challenges decided to true, decide game to false.
        /// if undecided challenge remains, revert.
        ///
        /// TODO: weight
        #[weight = 100_000]
        function settle_game(property: Property)
        {
            let origin = ensure_signed(origin)?;
            Self::only_from_dispute_contract(&origin, &property)?;

            let id = Self::get_property_id(&property);
            bytes32 id = utils.getPropertyId(_property);
            ensure!(
                Self::started(&id),
                Error::<T>::PropertyIsNotClaimed,
            );

            let mut game = Self::games(&id);
            ensure!(
                game.create_block < Self::block_number() - Self::DisputePeriod(),
                Error::<T>::DisputePeriodHasNotBeenPassed,
            );

            for challenge in game.challenges.iter() {
                let decision = Self::get_game(challenge)?.decision;
                if decision == Decision::True {
                    game.decision = Decision::False;
                    Self::deposit_event(RawEvent::PropertyDecided(id, false));
                    return Ok(());
                }
                ensure!(
                    decision == Decision::Undecided,
                    Error::<T>::UndecidedChallengeExists,
                );
            }
            game.decision = Decision::True;
            <Games<T>>::insert(&id, game);
            Self::deposit_event(RawEvent::PropertyDecided(id, true));
        }
    }
}

fn migrate<T: Trait>() {
    // TODO: When runtime upgrade, migrate stroage.
    // if let Some(current_era) = CurrentEra::get() {
    //     let history_depth = HistoryDepth::get();
    //     for era in current_era.saturating_sub(history_depth)..=current_era {
    //         ErasStartSessionIndex::migrate_key_from_blake(era);
    //     }
}

impl<T: Trait> Module<T> {
    fn calc_code_put_costs(code: &Vec<u8>) -> Weight {
        <Module<T>>::current_schedule()
            .put_code_per_byte_cost
            .saturating_mul(code.len() as Weight)
    }

    // ======= main ==========
    /// Perform a call to a specified contract.
    ///
    /// This function is similar to `Self::call`, but doesn't perform any address lookups and better
    /// suitable for calling directly from Rust.
    pub fn bare_call(
        origin: T::AccountId,
        dest: T::AccountId,
        input_data: Vec<u8>,
    ) -> ExecResult<T::ExternalCallError> {
        Self::execute_ovm(origin, |ctx| ctx.call(dest, input_data))
    }

    fn execute_ovm(
        origin: T::AccountId,
        func: impl FnOnce(
            &mut ExecutionContext<
                T,
                T::ExternalCallError,
                PredicateOvm,
                PredicateLoader,
                T::ExteralCall,
            >,
        ) -> ExecResult<T::ExternalCallError>,
    ) -> ExecResult<T::ExternalCallError> {
        let cfg = Config::preload::<T>();
        let vm = PredicateOvm::new(&cfg.schedule);
        let loader = PredicateLoader::new(&cfg.schedule);
        let mut ctx = ExecutionContext::top_level(origin.clone(), &cfg, &vm, &loader);

        func(&mut ctx)
    }

    // ======= callable ======
    /// Get of true/false the decision of property.
    pub fn is_decided(property: &PropertyOf<T>) -> Decision {
        let game = match Self::games(Self::get_property_id(property)) {
            Some(game) => game,
            None => return Decision::Undecided,
        };
        game.decision
    }

    /// Get of true/false the decision of game id.
    pub fn is_decided_by_id(id: Self::Hash) -> Decision {
        let game = match Self::games(&id) {
            Some(game) => game,
            None => return Decision::Undecided,
        };
        game.decision
    }

    /// Get of the instatiated challenge game from claim_id.
    pub fn get_game(claim_id: &T::Hash) -> Option<ChallengeGameOf<T>> {
        Self::games(claim_id)
    }

    /// Get of the property id from the propaty itself.
    pub fn get_property_id(property: &PropertyOf<T>) -> T::Hash {
        T::Hashing::hash_of(property)
    }

    pub fn is_challenge_of(property: &PropertyOf<T>, challenge_property: &PropertyOf<T>) -> bool {
        if let Some(game) = Self::get_game(Self::get_property_id(property)) {
            if let Some(_) =
                Self::find_index(&game.challenges, &Self::get_property_id(challenge_property))
            {
                return true;
            }
        }
        false
    }

    /// check if game of given id is already started.
    pub fn started(id: &T::Hash) -> bool {
        if let Some(game) = Self::games(id) {
            return game.created_block != 0;
        }
        false
    }

    // ======= helper =======
    pub fn block_number() -> <T as system::Trait>::BlockNumber {
        <system::Module<T>>::block_number()
    }

    pub fn is_decidable(property_id: &T::Hash) -> bool {
        let game = match Self::games(property_id) {
            Some(game) => game,
            None => return false,
        };

        if game.created_block > Self::block_number() - T::DisputePeriod::get() {
            return false;
        }

        // check all game.challenges should be false
        game.challenges.iter().all(|challenge| {
            if let Some(challenging_game) = Self::games(challenge) {
                return challenging_game.decision == Decision::False;
            }
            false
        })
    }

    fn create_game(id: T::Hash) -> ChallengeGameOf<T> {
        ChallengeGame {
            property_hash: id,
            /// challenges inputs
            challenges: vec![0],
            /// the result of this challenge.
            decision: Decision::Undecided,
            /// the block number when this was issued.
            created_block: Self::block_number(),
        }
    }

    fn get_decision(result: bool) -> Decision {
        if result {
            return Decision::True;
        }
        Decision::False
    }

    fn find_index<Hash: PartialEq, Eq>(array: &Vec<Hash>, item: &Hash) -> Option<usize> {
        array.iter().position(|hash| hash == item)
    }

    // ======= modifier =======
    fn only_from_dispute_contract(
        origin: &T::AccountId,
        property: &PropertyOf<T>,
    ) -> DispatchResult {
        ensure!(
            property.predicate_address == origin,
            Error::<T>::MustBeCalledFromPredicate,
        );
        Ok(())
    }
}
