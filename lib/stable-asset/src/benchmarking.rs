// This file is part of Acala.

// Copyright (C) 2020-2025 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<CurrencyId, Precision> {
	fn setup_assets_and_pool_asset(u: u32) -> Option<(Vec<CurrencyId>, Vec<Precision>, CurrencyId)>;
	fn setup_create_stable_pool(
		assets: Vec<CurrencyId>,
		precisions: Vec<Precision>,
		pool_asset: CurrencyId,
	) -> Option<StableAssetPoolId>;
}

impl<CurrencyId, Precision> BenchmarkHelper<CurrencyId, Precision> for () {
	fn setup_assets_and_pool_asset(_u: u32) -> Option<(Vec<CurrencyId>, Vec<Precision>, CurrencyId)> {
		None
	}
	fn setup_create_stable_pool(
		_assets: Vec<CurrencyId>,
		_precisions: Vec<Precision>,
		_pool_asset: CurrencyId,
	) -> Option<StableAssetPoolId> {
		None
	}
}

#[benchmarks(
	where
	T::Balance: From<u128>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_pool() {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(2).unwrap();

		let mint_fee = 10000000u128.into();
		let swap_fee = 20000000u128.into();
		let redeem_fee = 50000000u128.into();
		let initial_a = 10000u128.into();
		let fee_recipient: T::AccountId = account("fee", 0, 0);
		let yield_recipient: T::AccountId = account("yield", 1, 0);
		let precision = 1_000_000_000_000u128.into();

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			pool_asset,
			assets,
			precisions,
			mint_fee,
			swap_fee,
			redeem_fee,
			initial_a,
			fee_recipient,
			yield_recipient,
			precision,
		);
	}

	#[benchmark]
	fn modify_a() {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(2).unwrap();
		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Root, pool_id, 1000u128.into(), 2629112370u32.into());
	}

	#[benchmark]
	fn modify_fees() {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(2).unwrap();
		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			pool_id,
			Some(100u128.into()),
			Some(200u128.into()),
			Some(300u128.into()),
		);
	}

	#[benchmark]
	fn modify_recipients() {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(2).unwrap();
		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			pool_id,
			Some(account("fee-1", 3, 0)),
			Some(account("yield-1", 4, 0)),
		);
	}

	#[benchmark]
	fn mint(u: Liner<2, { T::PoolAssetLimit::get() }>) {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(u).unwrap();

		let caller: T::AccountId = account("caller", 0, 0);
		let amount = 1_000_000_000_000u128.into();

		let mut mint_args = vec![];
		for i in 0..u {
			let i_idx: usize = usize::try_from(i).unwrap();
			let currency_id = assets[i_idx];
			mint_args.push(amount);
			let _ = T::Assets::mint_into(currency_id, &caller, amount);
		}

		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), pool_id, mint_args, 0u128.into());
	}

	#[benchmark]
	fn swap(u: Liner<2, { T::PoolAssetLimit::get() }>) {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(u).unwrap();

		let caller: T::AccountId = account("caller", 0, 0);

		for i in 0..u {
			let i_idx: usize = usize::try_from(i).unwrap();
			let _ = T::Assets::mint_into(assets[i_idx], &caller, (u128::MAX / 100).into());
		}

		let amount = 1_000_000_000_000_000_000_000u128;
		let mint_args = match u {
			2 => vec![(amount / 10).into(), amount.into()],
			3 => vec![(amount / 10).into(), amount.into(), amount.into()],
			4 => vec![
				(amount / 100000).into(),
				(amount / 10000).into(),
				(amount / 10000).into(),
				(amount / 10000).into(),
			],
			5 => vec![
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
			],
			_ => vec![],
		};

		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		assert_ok!(Pallet::<T>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			pool_id,
			mint_args.clone(),
			0u128.into()
		));

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			pool_id,
			1,
			0,
			100000u128.into(),
			0u128.into(),
			u,
		);
	}

	#[benchmark]
	fn redeem_proportion(u: Liner<2, { T::PoolAssetLimit::get() }>) {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(u).unwrap();

		let caller: T::AccountId = account("caller", 0, 0);

		let mut mint_args = vec![];
		let mut redeem_args = vec![];
		for i in 0..u {
			let i_idx: usize = usize::try_from(i).unwrap();
			let currency_id = assets[i_idx];
			let amount = 1_000_000_000_000u128;
			let multiple: u128 = (i + 1).into();
			mint_args.push((1000 * amount * multiple).into());
			redeem_args.push(0u128.into());
			let _ = T::Assets::mint_into(currency_id, &caller, (u128::MAX / 100).into());
		}

		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		assert_ok!(Pallet::<T>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			pool_id,
			mint_args,
			0u128.into()
		));

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			pool_id,
			1_000_000_000_000u128.into(),
			redeem_args,
		);
	}

	#[benchmark]
	fn redeem_single(u: Liner<2, { T::PoolAssetLimit::get() }>) {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(u).unwrap();

		let caller: T::AccountId = account("caller", 0, 0);

		for i in 0..u {
			let i_idx: usize = usize::try_from(i).unwrap();
			let currency_id = assets[i_idx];
			let _ = T::Assets::mint_into(currency_id, &caller, (u128::MAX / 100).into());
		}

		let amount = 1_000_000_000_000_000_000_000u128;
		let mint_args = match u {
			2 => vec![(amount / 10).into(), amount.into()],
			3 => vec![(amount / 10).into(), amount.into(), amount.into()],
			4 => vec![
				(amount / 100000).into(),
				(amount / 10000).into(),
				(amount / 10000).into(),
				(amount / 10000).into(),
			],
			5 => vec![
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
				(amount / 100000000).into(),
			],
			_ => vec![],
		};

		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		assert_ok!(Pallet::<T>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			pool_id,
			mint_args,
			0u128.into()
		));

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			pool_id,
			10_000u128.into(),
			0u32,
			0u128.into(),
			u,
		);
	}

	#[benchmark]
	fn redeem_multi(u: Liner<2, { T::PoolAssetLimit::get() }>) {
		let (assets, precisions, pool_asset) = T::BenchmarkHelper::setup_assets_and_pool_asset(u).unwrap();

		let caller: T::AccountId = account("caller", 0, 0);

		let mut mint_args = vec![];
		let mut redeem_args = vec![];
		for i in 0..u {
			let i_idx: usize = usize::try_from(i).unwrap();
			let currency_id = assets[i_idx];
			let amount = 1_000_000_000_000u128;
			mint_args.push((100 * amount).into());
			redeem_args.push(amount.into());
			let _ = T::Assets::mint_into(currency_id, &caller, (u128::MAX / 100).into());
		}

		let pool_id = T::BenchmarkHelper::setup_create_stable_pool(assets, precisions, pool_asset).unwrap();

		assert_ok!(Pallet::<T>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			pool_id,
			mint_args,
			0u128.into()
		));

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			pool_id,
			redeem_args,
			(u128::MAX / 10u128).into(),
		);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
