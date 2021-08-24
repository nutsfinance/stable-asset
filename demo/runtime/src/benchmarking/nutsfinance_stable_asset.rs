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

use crate::{AccountId, Runtime, StableAsset};

use crate::benchmarking::utils::initialize_assets;
use frame_benchmarking::{account, whitelisted_caller};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

const SEED: u32 = 0;
const POOL_ASSET: u32 = 0u32;
const ASSET_A: u32 = 1u32;
const ASSET_B: u32 = 2u32;

runtime_benchmarks! {
	{ Runtime, nutsfinance_stable_asset }

	create_pool {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
	}: _(RawOrigin::Signed(tester), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient)

	modify_a {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
	}: _(RawOrigin::Signed(tester), pool_id, 1000u128, 2629112370)

	mint {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
	}: _(RawOrigin::Signed(tester), pool_id, vec![10000000u128, 20000000u128], 0u128)

	swap {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
		let _ = StableAsset::mint(RawOrigin::Signed(tester.clone()).into(), pool_id, vec![10000000u128, 20000000u128], 0u128);
	}: _(RawOrigin::Signed(tester), pool_id, 0, 1, 5000000u128, 0u128)

	collect_fee {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
		let _ = StableAsset::mint(RawOrigin::Signed(tester.clone()).into(), pool_id, vec![10000000u128, 20000000u128], 0u128);
		let _ = StableAsset::swap(RawOrigin::Signed(tester.clone()).into(), pool_id, 0, 1, 5000000u128, 0u128);
	}: _(RawOrigin::Signed(tester), pool_id)

	redeem_proportion {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
		let _ = StableAsset::mint(RawOrigin::Signed(tester.clone()).into(), pool_id, vec![10000000u128, 20000000u128], 0u128);
	}: _(RawOrigin::Signed(tester), pool_id, 10000000000000000u128, vec![0u128, 0u128])

	redeem_single {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
		let _ = StableAsset::mint(RawOrigin::Signed(tester.clone()).into(), pool_id, vec![10000000u128, 20000000u128], 0u128);
	}: _(RawOrigin::Signed(tester), pool_id, 10000000000000000u128, 0u32, 0u128)

	redeem_multi {
		let tester: AccountId = whitelisted_caller();
		let pool_asset = POOL_ASSET;
		let assets = vec![ASSET_A, ASSET_B];
		let precisions = vec![10000000000u128, 10000000000u128];
		let mint_fee = 10000000u128;
		let swap_fee = 20000000u128;
		let redeem_fee = 50000000u128;
		let intial_a = 100u128;
		let fee_recipient: AccountId = account("fee", 0, SEED);
		let _ = StableAsset::create_pool(RawOrigin::Signed(tester.clone()).into(), pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient.clone());
		let pool_id = StableAsset::pool_count() - 1;
		let _ = initialize_assets(tester.clone(), fee_recipient.clone(), POOL_ASSET, ASSET_A, ASSET_B)?;
		let _ = StableAsset::mint(RawOrigin::Signed(tester.clone()).into(), pool_id, vec![10000000u128, 20000000u128], 0u128);
	}: _(RawOrigin::Signed(tester), pool_id, vec![500000u128, 500000u128], 1100000000000000000u128)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use orml_benchmarking::impl_benchmark_test_suite;

	impl_benchmark_test_suite!(new_test_ext(),);
}
