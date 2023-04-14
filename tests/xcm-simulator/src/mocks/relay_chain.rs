// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use frame_support::{
    construct_runtime, parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU32, Everything, Nothing},
    weights::Weight,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};

use polkadot_parachain::primitives::Id as ParaId;
use polkadot_runtime_parachains::{configuration, origin, shared, ump};
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, ChildParachainAsNative,
    ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
    CurrencyAdapter as XcmCurrencyAdapter, FixedRateOfFungible, FixedWeightBounds, IsConcrete,
    SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::XcmExecutor;

pub type AccountId = AccountId32;
pub type Balance = u128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = Everything;
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

impl shared::Config for Runtime {}

impl configuration::Config for Runtime {
    type WeightInfo = configuration::TestWeightInfo;
}

parameter_types! {
    pub const KsmLocation: MultiLocation = Here.into_location();
    pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
    pub UniversalLocation: InteriorMultiLocation = Here;
}

pub type SovereignAccountOf = (
    ChildParachainConvertsVia<ParaId, AccountId>,
    AccountId32Aliases<KusamaNetwork, AccountId>,
);

pub type LocalAssetTransactor =
    XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;

type LocalOriginConverter = (
    SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
    ChildParachainAsNative<origin::Origin, RuntimeOrigin>,
    SignedAccountId32AsNative<KusamaNetwork, RuntimeOrigin>,
    ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
);

parameter_types! {
    pub const BaseXcmWeight: Weight = Weight::from_ref_time(1_000);
    pub KsmPerSecond: (AssetId, u128, u128) = (Concrete(KsmLocation::get()), 1, 1024 * 1024);
    pub const MaxInstructions: u32 = 100;
}

pub type XcmRouter = super::RelayChainXcmRouter;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = LocalOriginConverter;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
    type ResponseHandler = ();
    type AssetTrap = ();
    type AssetClaims = ();
    type SubscriptionService = ();

    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, KusamaNetwork>;

pub type LocationToAccountId = (ChildParachainConvertsVia<ParaId, AccountId>,);

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    // Anyone can execute XCM messages locally...
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = pallet_xcm::TestWeightInfo;
}

parameter_types! {
    pub const FirstMessageFactorPercent: u64 = 100;
}

impl ump::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type UmpSink = ump::XcmSink<XcmExecutor<XcmConfig>, Runtime>;
    type FirstMessageFactorPercent = FirstMessageFactorPercent;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = ump::TestWeightInfo;
}

impl origin::Config for Runtime {}

impl pallet_uniques::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type CollectionDeposit = frame_support::traits::ConstU128<1_000>;
    type ItemDeposit = frame_support::traits::ConstU128<1_000>;
    type MetadataDepositBase = frame_support::traits::ConstU128<1_000>;
    type AttributeDepositBase = frame_support::traits::ConstU128<1_000>;
    type DepositPerByte = frame_support::traits::ConstU128<1>;
    type StringLimit = frame_support::traits::ConstU32<64>;
    type KeyLimit = frame_support::traits::ConstU32<64>;
    type ValueLimit = frame_support::traits::ConstU32<128>;
    type Locker = ();
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        ParasOrigin: origin::{Pallet, Origin},
        ParasUmp: ump::{Pallet, Call, Storage, Event},
        XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
        Uniques: pallet_uniques::{Pallet, Call, Storage, Event<T>},
    }
);
