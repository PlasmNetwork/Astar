#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::{Codec, Decode, Encode};

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

pub mod default;
pub mod events;
pub mod traits;

pub type Result<T> = core::result::Result<T, &'static str>;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Range<I: traits::SimpleArithmetic + traits::Member + Codec> {
    pub start: I,
    pub end: I,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct StateObject<T: traits::Member + Codec> {
    pub predicate: AccountId,
    pub data: T,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct StateUpdate<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub range: Range<I>,
    pub state_object: StateObject<T>,
    pub plasma_block_number: BlockNumber,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Checkpoint<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub state_update: StateUpdate<T, I>,
    pub sub_range: Range<I>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Transaction<
    U: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub deposit_contract: AccountId,
    pub range: Range<I>,
    pub body: U,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Challenge<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub challenged_checkpoint: Checkpoint<T, I>,
    pub challenging_checkpoint: Checkpoint<T, I>,
}

pub fn keccak256<E: Encode>(data: &E) -> Hash {
	Hash::decode(&mut &ink_utils::hash::keccak256(&data.encode()[..])[..]).expect("Hash decoded error in keccak256.")
}
