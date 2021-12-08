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

use crate::{AccountId, AssetId, Assets, Runtime, StableAssetPalletId};

use frame_system::RawOrigin;
use sp_runtime::{traits::AccountIdConversion, DispatchResult};
use sp_std::prelude::*;

pub fn initialize_assets(
	tester: AccountId,
	fee_recipient: AccountId,
	pool_asset: AssetId,
	assets: Vec<AssetId>,
) -> DispatchResult {
	frame_system::Pallet::<Runtime>::inc_providers(&tester);
	frame_system::Pallet::<Runtime>::inc_providers(&fee_recipient);
	let _ = Assets::create(
		RawOrigin::Signed(tester.clone()).into(),
		pool_asset,
		sp_runtime::MultiAddress::Id(tester.clone()),
		1,
	)?;
	for asset in &assets {
		let _ = Assets::create(
			RawOrigin::Signed(tester.clone()).into(),
			*asset,
			sp_runtime::MultiAddress::Id(tester.clone()),
			1,
		)?;
		let _ = Assets::mint(
			RawOrigin::Signed(tester.clone()).into(),
			*asset,
			sp_runtime::MultiAddress::Id(tester.clone()),
			100000000,
		)?;
	}
	let pallet_id: AccountId = StableAssetPalletId::get().into_account();
	let _ = Assets::set_team(
		RawOrigin::Signed(tester.clone()).into(),
		pool_asset,
		sp_runtime::MultiAddress::Id(pallet_id.clone()),
		sp_runtime::MultiAddress::Id(pallet_id.clone()),
		sp_runtime::MultiAddress::Id(tester.clone()),
	);
	Ok(().into())
}

#[cfg(test)]
pub mod tests {
	pub fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<crate::Runtime>()
			.unwrap()
			.into()
	}
}
