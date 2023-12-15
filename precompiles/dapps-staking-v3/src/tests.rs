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

extern crate alloc;
use crate::{mock::*, *};
use fp_evm::ExitError;
use frame_support::assert_ok;
use precompile_utils::testing::*;
use sp_core::H160;
use sp_runtime::{traits::Zero, AccountId32, Perbill};

use pallet_dapp_staking_v3::{AccountLedger, ActiveProtocolState, EraNumber, EraRewards};

fn precompiles() -> DappStakingPrecompile<Test> {
    PrecompilesValue::get()
}

#[test]
fn read_current_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_current_era {},
            )
            .expect_no_logs()
            .execute_returns(ActiveProtocolState::<Test>::get().era);

        // advance a few eras, check value again
        advance_to_era(7);
        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_current_era {},
            )
            .expect_no_logs()
            .execute_returns(ActiveProtocolState::<Test>::get().era);
    });
}

#[test]
fn read_unbonding_period_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let unlocking_period_in_eras: EraNumber =
            <Test as pallet_dapp_staking_v3::Config>::UnlockingPeriod::get();

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_unbonding_period {},
            )
            .expect_no_logs()
            .execute_returns(unlocking_period_in_eras);
    });
}

#[test]
fn read_era_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Check historic era for rewards
        let era = 3;
        advance_to_era(era + 1);

        let span_index = DAppStaking::<Test>::era_reward_span_index(era);

        let era_rewards_span = EraRewards::<Test>::get(span_index).expect("Entry must exist.");
        let expected_reward = era_rewards_span
            .get(era)
            .map(|r| r.staker_reward_pool + r.dapp_reward_pool)
            .expect("It's history era so it must exist.");
        assert!(expected_reward > 0, "Sanity check.");

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_era_reward { era },
            )
            .expect_no_logs()
            .execute_returns(expected_reward);

        // Check current era for rewards, must be zero
        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_era_reward { era: era + 1 },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());
    });
}

#[test]
fn read_era_staked_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_h160 = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_h160);
        let amount = register_and_stake(staker_h160, smart_contract.clone());
        let anchor_era = ActiveProtocolState::<Test>::get().era;

        // 1. Current era stake must be zero, since stake is only valid from the next era.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked { era: anchor_era },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());

        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: anchor_era + 1,
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 2. Advance to next era, and check next era after the anchor.
        advance_to_era(anchor_era + 5);
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: anchor_era + 1,
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 3. Check era after the next one, must throw an error.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: ActiveProtocolState::<Test>::get().era + 2,
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Era is in the future");
    });
}

#[test]
fn read_staked_amount_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let dynamic_addresses = into_dynamic_addresses(staker_h160);

        // 1. Sanity checks - must be zero before anything is staked.
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }

        // 2. Stake some amount and check again
        let smart_contract_h160 = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_h160);
        let amount = register_and_stake(staker_h160, smart_contract.clone());
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(amount);
        }

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }
    });
}

#[test]
fn read_staked_amount_on_contract_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_h160 = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_h160);
        let dynamic_addresses = into_dynamic_addresses(staker_h160);

        // 1. Sanity checks - must be zero before anything is staked.
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_h160.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }

        // 2. Stake some amount and check again
        let amount = register_and_stake(staker_h160, smart_contract.clone());
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_h160.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(amount);
        }

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_h160.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }
    });
}

#[test]
fn read_contract_stake_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_h160 = H160::repeat_byte(0xFA);

        // 1. Sanity checks - must be zero before anything is staked.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_h160.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());

        // 2. Stake some amount and check again
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_h160);
        let amount = register_and_stake(staker_h160, smart_contract.clone());

        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_h160.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_h160.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());
    });
}

// #[test]
// fn register_via_precompile_fails() {
//     ExternalityBuilder::default()
//         .with_balances(vec![(TestAccount::Alex.into(), 200 * AST)])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             precompiles()
//                 .prepare_test(
//                     TestAccount::Alex,
//                     precompile_address(),
//                     EvmDataWriter::new_with_selector(Action::Register)
//                         .write(Address(TEST_CONTRACT.clone()))
//                         .build(),
//                 )
//                 .expect_no_logs()
//                 .execute_error(ExitError::Other(alloc::borrow::Cow::Borrowed(
//                     "register via evm precompile is not allowed",
//                 )));
//         });
// }

// #[test]
// fn bond_and_stake_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 100 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             register_and_verify(TestAccount::Alex, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = 50 * AST;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Bobo.into(), amount_staked_bobo);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);
//         });
// }

// #[test]
// fn unbond_and_unstake_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 100 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex.into();
//             register_and_verify(developer, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);
//             let amount_staked_dino = 50 * AST;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // Bobo unstakes all
//             let era = 2;
//             advance_to_era(era);
//             unbond_unstake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             contract_era_stake_verify(TEST_CONTRACT, amount_staked_dino);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Dino, amount_staked_dino);

//             // withdraw unbonded funds
//             advance_to_era(era + UNBONDING_PERIOD + 1);
//             withdraw_unbonded_verify(TestAccount::Bobo);
//         });
// }

// #[test]
// fn claim_dapp_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex;
//             register_and_verify(developer, TEST_CONTRACT);

//             let stake_amount_total = 300 * AST;
//             let ratio_bobo = Perbill::from_rational(3u32, 5u32);
//             let ratio_dino = Perbill::from_rational(2u32, 5u32);
//             let amount_staked_bobo = ratio_bobo * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = ratio_dino * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // advance era and claim reward
//             let era = 5;
//             advance_to_era(era);
//             claim_dapp_and_verify(TEST_CONTRACT, era - 1);

//             //check that the reward is payed out to the developer
//             let developer_reward = DAPP_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;
//             assert_eq!(
//                 <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                     &TestAccount::Alex.into()
//                 ),
//                 (200 * AST) + developer_reward - REGISTER_DEPOSIT
//             );
//         });
// }

// #[test]
// fn claim_staker_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex;
//             register_and_verify(developer, TEST_CONTRACT);

//             let stake_amount_total = 300 * AST;
//             let ratio_bobo = Perbill::from_rational(3u32, 5u32);
//             let ratio_dino = Perbill::from_rational(2u32, 5u32);
//             let amount_staked_bobo = ratio_bobo * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = ratio_dino * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // advance era and claim reward
//             advance_to_era(5);

//             let stakers_reward = STAKER_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;

//             // Ensure that all rewards can be claimed for the first staker
//             for era in 1..DappsStaking::current_era() as Balance {
//                 claim_staker_and_verify(TestAccount::Bobo, TEST_CONTRACT);
//                 assert_eq!(
//                     <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                         &TestAccount::Bobo.into()
//                     ),
//                     (200 * AST) + ratio_bobo * stakers_reward * era
//                 );
//             }

//             // Repeat the same thing for the second staker
//             for era in 1..DappsStaking::current_era() as Balance {
//                 claim_staker_and_verify(TestAccount::Dino, TEST_CONTRACT);
//                 assert_eq!(
//                     <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                         &TestAccount::Dino.into()
//                     ),
//                     (200 * AST) + ratio_dino * stakers_reward * era
//                 );
//             }
//         });
// }

// #[test]
// fn set_reward_destination() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();
//             // register contract and stake it
//             register_and_verify(TestAccount::Alex.into(), TEST_CONTRACT);

//             // bond & stake the origin contract
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, 100 * AST);

//             // change destinations and verfiy it was successful
//             set_reward_destination_verify(TestAccount::Bobo.into(), RewardDestination::FreeBalance);
//             set_reward_destination_verify(
//                 TestAccount::Bobo.into(),
//                 RewardDestination::StakeBalance,
//             );
//             set_reward_destination_verify(TestAccount::Bobo.into(), RewardDestination::FreeBalance);
//         });
// }

// #[test]
// fn withdraw_from_unregistered() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex.into();
//             register_and_verify(developer, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let contract_id =
//                 decode_smart_contract_v1_from_array(TEST_CONTRACT.clone().to_fixed_bytes()).unwrap();
//             assert_ok!(DappsStaking::unregister(RuntimeOrigin::root(), contract_id));

//             withdraw_from_unregistered_verify(TestAccount::Bobo.into(), TEST_CONTRACT);
//         });
// }

// #[test]
// fn nomination_transfer() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register two contracts for nomination transfer test
//             let origin_contract = H160::repeat_byte(0x09);
//             let target_contract = H160::repeat_byte(0x0A);
//             register_and_verify(TestAccount::Alex.into(), origin_contract);
//             register_and_verify(TestAccount::Dino.into(), target_contract);

//             // bond & stake the origin contract
//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, origin_contract, amount_staked_bobo);

//             // transfer nomination and ensure it was successful
//             nomination_transfer_verify(
//                 TestAccount::Bobo,
//                 origin_contract,
//                 10 * AST,
//                 target_contract,
//             );
//         });
// }

// // ****************************************************************************************************
// // Helper functions
// // ****************************************************************************************************

// /// helper function to register and verify if registration is valid
// fn register_and_verify(developer: TestAccount, contract: H160) {
//     let smart_contract =
//         decode_smart_contract_v1_from_array(contract.clone().to_fixed_bytes()).unwrap();
//     DappsStaking::register(
//         RuntimeOrigin::root(),
//         developer.clone().into(),
//         smart_contract,
//     )
//     .unwrap();

//     // check the storage after the register
//     let dev_account_id: AccountId32 = developer.into();
//     let smart_contract_bytes =
//         (DappsStaking::registered_contract(dev_account_id).unwrap_or_default()).encode();

//     assert_eq!(
//         // 0-th byte is enum value discriminator
//         smart_contract_bytes[1..21],
//         contract.to_fixed_bytes()
//     );
// }

// /// helper function to read ledger storage item
// fn read_staked_amount_h160_verify(staker: TestAccount, amount: u128) {
//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ReadStakedAmount)
//                 .write(Bytes(
//                     Into::<H160>::into(staker.clone()).to_fixed_bytes().to_vec(),
//                 ))
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(amount).build());
// }

// /// helper function to read ledger storage item for ss58 account
// fn read_staked_amount_ss58_verify(staker: TestAccount, amount: u128) {
//     let staker_acc_id: AccountId32 = staker.clone().into();

//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ReadStakedAmount)
//                 .write(Bytes(staker_acc_id.encode()))
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(amount).build());
// }

// /// helper function to bond, stake and verify if resulet is OK
// fn bond_stake_and_verify(staker: TestAccount, contract: H160, amount: u128) {
//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::BondAndStake)
//                 .write(Address(contract.clone()))
//                 .write(amount)
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());

//     read_staked_amount_h160_verify(staker.clone(), amount);
//     read_staked_amount_ss58_verify(staker, amount);
// }

// /// helper function to unbond, unstake and verify if result is OK
// fn unbond_unstake_and_verify(staker: TestAccount, contract: H160, amount: u128) {
//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::UnbondAndUnstake)
//                 .write(Address(contract.clone()))
//                 .write(amount)
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());
// }

// /// helper function to withdraw unstaked funds and verify if result is OK
// fn withdraw_unbonded_verify(staker: TestAccount) {
//     let staker_acc_id = AccountId32::from(staker.clone());

//     // call unbond_and_unstake(). Check usable_balance before and after the call
//     assert_ne!(
//         <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker_acc_id),
//         <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker_acc_id)
//     );

//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::WithdrawUnbounded).build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());

//     assert_eq!(
//         <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker_acc_id),
//         <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker_acc_id)
//     );
// }

// /// helper function to verify change of reward destination for a staker
// fn set_reward_destination_verify(staker: TestAccount, reward_destination: RewardDestination) {
//     // Read staker's ledger
//     let staker_acc_id = AccountId32::from(staker.clone());
//     let init_ledger = DappsStaking::ledger(&staker_acc_id);
//     // Ensure that something is staked or being unbonded
//     assert!(!init_ledger.is_empty());

//     let reward_destination_raw: u8 = match reward_destination {
//         RewardDestination::FreeBalance => 0,
//         RewardDestination::StakeBalance => 1,
//     };
//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::SetRewardDestination)
//                 .write(reward_destination_raw)
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());

//     let final_ledger = DappsStaking::ledger(&staker_acc_id);
//     assert_eq!(final_ledger.reward_destination(), reward_destination);
// }

// /// helper function to withdraw funds from unregistered contract
// fn withdraw_from_unregistered_verify(staker: TestAccount, contract: H160) {
//     let smart_contract =
//         decode_smart_contract_v1_from_array(contract.clone().to_fixed_bytes()).unwrap();
//     let staker_acc_id = AccountId32::from(staker.clone());
//     let init_staker_info = DappsStaking::staker_info(&staker_acc_id, &smart_contract);
//     assert!(!init_staker_info.latest_staked_value().is_zero());

//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::WithdrawFromUnregistered)
//                 .write(Address(contract.clone()))
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());

//     let final_staker_info = DappsStaking::staker_info(&staker_acc_id, &smart_contract);
//     assert!(final_staker_info.latest_staked_value().is_zero());
// }

// /// helper function to verify nomination transfer from origin to target contract
// fn nomination_transfer_verify(
//     staker: TestAccount,
//     origin_contract: H160,
//     amount: Balance,
//     target_contract: H160,
// ) {
//     let origin_smart_contract =
//         decode_smart_contract_v1_from_array(origin_contract.clone().to_fixed_bytes()).unwrap();
//     let target_smart_contract =
//         decode_smart_contract_v1_from_array(target_contract.clone().to_fixed_bytes()).unwrap();
//     let staker_acc_id = AccountId32::from(staker.clone());

//     // Read init data staker info states
//     let init_origin_staker_info = DappsStaking::staker_info(&staker_acc_id, &origin_smart_contract);
//     let init_target_staker_info = DappsStaking::staker_info(&staker_acc_id, &target_smart_contract);

//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::NominationTransfer)
//                 .write(Address(origin_contract.clone()))
//                 .write(amount)
//                 .write(Address(target_contract.clone()))
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());

//     let final_origin_staker_info =
//         DappsStaking::staker_info(&staker_acc_id, &origin_smart_contract);
//     let final_target_staker_info =
//         DappsStaking::staker_info(&staker_acc_id, &target_smart_contract);

//     // Verify final state
//     let will_be_unstaked = init_origin_staker_info
//         .latest_staked_value()
//         .saturating_sub(amount)
//         < MINIMUM_STAKING_AMOUNT;
//     let transfer_amount = if will_be_unstaked {
//         init_origin_staker_info.latest_staked_value()
//     } else {
//         amount
//     };

//     assert_eq!(
//         final_origin_staker_info.latest_staked_value() + transfer_amount,
//         init_origin_staker_info.latest_staked_value()
//     );
//     assert_eq!(
//         final_target_staker_info.latest_staked_value() - transfer_amount,
//         init_target_staker_info.latest_staked_value()
//     );
// }

// /// helper function to bond, stake and verify if result is OK
// fn claim_dapp_and_verify(contract: H160, era: EraIndex) {
//     precompiles()
//         .prepare_test(
//             TestAccount::Bobo,
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ClaimDapp)
//                 .write(Address(contract.clone()))
//                 .write(era)
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());
// }

// /// helper function to bond, stake and verify if the result is OK
// fn claim_staker_and_verify(staker: TestAccount, contract: H160) {
//     precompiles()
//         .prepare_test(
//             staker,
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ClaimStaker)
//                 .write(Address(contract.clone()))
//                 .build(),
//         )
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(true).build());
// }

// fn contract_era_stake_verify(contract: H160, amount: Balance) {
//     precompiles()
//         .prepare_test(
//             TestAccount::Alex,
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ReadContractStake)
//                 .write(Address(contract.clone()))
//                 .build(),
//         )
//         .expect_cost(2 * READ_WEIGHT)
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(amount).build());
// }

// /// helper function to verify latest staked amount
// fn verify_staked_amount(contract: H160, staker: TestAccount, amount: Balance) {
//     precompiles()
//         .prepare_test(
//             staker.clone(),
//             precompile_address(),
//             EvmDataWriter::new_with_selector(Action::ReadStakedAmountOnContract)
//                 .write(Address(contract.clone()))
//                 .write(Bytes(
//                     Into::<H160>::into(staker.clone()).to_fixed_bytes().to_vec(),
//                 ))
//                 .build(),
//         )
//         .expect_cost(READ_WEIGHT)
//         .expect_no_logs()
//         .execute_returns(EvmDataWriter::new().write(amount).build());
// }

// /// Helper method to decode type SmartContract enum from [u8; 20]
// fn decode_smart_contract_v1_from_array(
//     contract_array: [u8; 20],
// ) -> Result<<TestRuntime as pallet_dapps_staking::Config>::SmartContract, String> {
//     // Encode contract address to fit SmartContract enum.
//     let mut contract_enum_encoded: [u8; 21] = [0; 21];
//     contract_enum_encoded[0] = 0; // enum for EVM H160 address is 0
//     contract_enum_encoded[1..21].copy_from_slice(&contract_array);

//     let smart_contract = <TestRuntime as pallet_dapps_staking::Config>::SmartContract::decode(
//         &mut &contract_enum_encoded[..21],
//     )
//     .map_err(|_| "Error while decoding SmartContract")?;

//     Ok(smart_contract)
// }
