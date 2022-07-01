// This file is part of NUTS Finance.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{mock::*, Error, StableAssetXcmPoolInfo};
use frame_support::assert_noop;
use frame_support::assert_ok;

use frame_support::traits::fungibles::Inspect;
use sp_std::collections::btree_map::BTreeMap;

pub const BALANCE_OFF: u128 = 1;

fn last_event() -> Event {
	frame_system::pallet::Pallet::<Test>::events()
		.pop()
		.expect("Event expected")
		.event
}

fn create_pool() -> i64 {
	let pool_asset = TestAssets::create_asset().expect("asset should be created");
	assert_ok!(StableAsset::create_pool(Origin::signed(1), pool_asset,));
	pool_asset
}

#[test]
fn create_pool_successful() {
	new_test_ext().execute_with(|| {
		assert_eq!(StableAsset::pool_count(), 0);
		assert_ok!(StableAsset::create_pool(Origin::signed(1), 1,));
		assert_eq!(
			StableAsset::pools(0),
			Some(StableAssetXcmPoolInfo {
				pool_asset: 1,
				balances: BTreeMap::new(),
				limits: BTreeMap::new(),
			})
		);
	});
}

#[test]
fn update_limit_successful() {
	new_test_ext().execute_with(|| {
		create_pool();
		System::set_block_number(2);
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 100));
		assert_eq!(
			StableAsset::pools(0),
			Some(StableAssetXcmPoolInfo {
				pool_asset: 0,
				balances: BTreeMap::new(),
				limits: BTreeMap::from([((1u32, 2u32), 100u128)]),
			})
		);
		if let Event::StableAsset(crate::pallet::Event::LimitUpdated {
			local_pool_id,
			chain_id,
			remote_pool_id,
			limit,
		}) = last_event()
		{
			assert_eq!(local_pool_id, 0);
			assert_eq!(chain_id, 1);
			assert_eq!(remote_pool_id, 2);
			assert_eq!(limit, 100);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn update_limit_pool_not_found() {
	new_test_ext().execute_with(|| {
		create_pool();
		assert_noop!(
			StableAsset::update_limit(Origin::signed(1), 1, 1, 2, 100),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn update_limit_failed_lower() {
	new_test_ext().execute_with(|| {
		create_pool();
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 100));
		assert_noop!(
			StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 50),
			Error::<Test>::NewLimitInvalid
		);
	});
}

#[test]
fn mint_successful() {
	new_test_ext().execute_with(|| {
		let asset_id = create_pool();
		System::set_block_number(2);
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 10000));
		assert_ok!(StableAsset::mint(Origin::signed(2), 2, 0, 1, 2, 200));
		assert_eq!(
			StableAsset::pools(0),
			Some(StableAssetXcmPoolInfo {
				pool_asset: 0,
				balances: BTreeMap::from([((1u32, 2u32), 200u128)]),
				limits: BTreeMap::from([((1u32, 2u32), 10000u128)]),
			})
		);
		assert_eq!(TestAssets::balance(asset_id, &2), 200 - BALANCE_OFF);
		if let Event::StableAsset(crate::pallet::Event::Minted {
			minter,
			local_pool_id,
			chain_id,
			remote_pool_id,
			mint_amount,
		}) = last_event()
		{
			assert_eq!(minter, 2);
			assert_eq!(local_pool_id, 0);
			assert_eq!(chain_id, 1);
			assert_eq!(remote_pool_id, 2);
			assert_eq!(mint_amount, 200);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn mint_pool_not_found() {
	new_test_ext().execute_with(|| {
		create_pool();
		assert_noop!(
			StableAsset::mint(Origin::signed(2), 2, 1, 1, 2, 200),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn mint_over_limit() {
	new_test_ext().execute_with(|| {
		let _asset_id = create_pool();
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 100));
		assert_noop!(
			StableAsset::mint(Origin::signed(2), 2, 0, 1, 2, 200),
			Error::<Test>::MintOverLimit
		);
	});
}

#[test]
fn redeem_proportion_successful() {
	new_test_ext().execute_with(|| {
		let asset_id = create_pool();
		System::set_block_number(2);
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 100000));
		assert_ok!(StableAsset::mint(Origin::signed(2), 2, 0, 1, 2, 20000));
		assert_ok!(StableAsset::redeem_proportion(
			Origin::signed(2),
			0,
			1,
			2,
			10000,
			vec![0u128, 0u128]
		));
		assert_eq!(TestAssets::balance(asset_id, &2), 10000 - BALANCE_OFF);
		if let Event::StableAsset(crate::pallet::Event::RedeemedProportion {
			redeemer,
			local_pool_id,
			chain_id,
			remote_pool_id,
			input_amount,
			min_redeem_amounts,
		}) = last_event()
		{
			assert_eq!(redeemer, 2);
			assert_eq!(local_pool_id, 0);
			assert_eq!(chain_id, 1);
			assert_eq!(remote_pool_id, 2);
			assert_eq!(input_amount, 10000);
			assert_eq!(min_redeem_amounts, vec![0u128, 0u128]);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn redeem_single_successful() {
	new_test_ext().execute_with(|| {
		let asset_id = create_pool();
		System::set_block_number(2);
		assert_ok!(StableAsset::update_limit(Origin::signed(1), 0, 1, 2, 100000));
		assert_ok!(StableAsset::mint(Origin::signed(2), 2, 0, 1, 2, 20000));
		assert_ok!(StableAsset::redeem_single(
			Origin::signed(2),
			0,
			1,
			2,
			10000,
			1,
			0u128,
			2
		));
		assert_eq!(TestAssets::balance(asset_id, &2), 10000 - BALANCE_OFF);
		if let Event::StableAsset(crate::pallet::Event::RedeemedSingle {
			redeemer,
			local_pool_id,
			chain_id,
			remote_pool_id,
			input_amount,
			i,
			min_redeem_amount,
		}) = last_event()
		{
			assert_eq!(redeemer, 2);
			assert_eq!(local_pool_id, 0);
			assert_eq!(chain_id, 1);
			assert_eq!(remote_pool_id, 2);
			assert_eq!(input_amount, 10000);
			assert_eq!(i, 1);
			assert_eq!(min_redeem_amount, 0u128);
		} else {
			panic!("Unexpected event");
		}
	});
}
