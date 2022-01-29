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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

extern crate sp_runtime;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

use crate::traits::StableAsset;
use frame_support::codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use frame_support::ensure;
use frame_support::traits::fungibles::{Inspect, Mutate, Transfer};
use frame_support::{traits::Get, weights::Weight};
use scale_info::TypeInfo;
use sp_runtime::traits::{AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero};
use sp_std::prelude::*;

pub type PoolTokenIndex = u32;

pub type StableAssetPoolId = u32;

const NUMBER_OF_ITERATIONS_TO_CONVERGE: i32 = 255; // the number of iterations to sum d and y

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub struct StableAssetPoolInfo<AssetId, AtLeast64BitUnsigned, Balance, AccountId, BlockNumber> {
	pool_asset: AssetId,
	assets: Vec<AssetId>,
	precisions: Vec<AtLeast64BitUnsigned>,
	mint_fee: AtLeast64BitUnsigned,
	swap_fee: AtLeast64BitUnsigned,
	redeem_fee: AtLeast64BitUnsigned,
	total_supply: Balance,
	a: AtLeast64BitUnsigned,
	a_block: BlockNumber,
	future_a: AtLeast64BitUnsigned,
	future_a_block: BlockNumber,
	balances: Vec<Balance>,
	fee_recipient: AccountId,
	account_id: AccountId,
	yield_recipient: AccountId,
	precision: AtLeast64BitUnsigned,
}

pub trait WeightInfo {
	fn create_pool() -> Weight;
	fn modify_a() -> Weight;
	fn mint(u: u32) -> Weight;
	fn swap(u: u32) -> Weight;
	fn redeem_proportion(u: u32) -> Weight;
	fn redeem_single(u: u32) -> Weight;
	fn redeem_multi(u: u32) -> Weight;
}

pub mod traits {
	use crate::{PoolTokenIndex, StableAssetPoolId, StableAssetPoolInfo};
	use frame_support::dispatch::DispatchResult;
	use sp_std::prelude::*;

	pub trait ValidateAssetId<AssetId> {
		fn validate(a: AssetId) -> bool;
	}

	pub trait StableAsset {
		type AssetId;
		type AtLeast64BitUnsigned;
		type Balance;
		type AccountId;
		type BlockNumber;

		fn pool_count() -> StableAssetPoolId;

		fn pool(
			id: StableAssetPoolId,
		) -> Option<
			StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		>;

		fn create_pool(
			pool_asset: Self::AssetId,
			assets: Vec<Self::AssetId>,
			precisions: Vec<Self::AtLeast64BitUnsigned>,
			mint_fee: Self::AtLeast64BitUnsigned,
			swap_fee: Self::AtLeast64BitUnsigned,
			redeem_fee: Self::AtLeast64BitUnsigned,
			initial_a: Self::AtLeast64BitUnsigned,
			fee_recipient: Self::AccountId,
			yield_recipient: Self::AccountId,
			precision: Self::AtLeast64BitUnsigned,
		) -> DispatchResult;

		fn mint(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amounts: Vec<Self::Balance>,
			min_mint_amount: Self::Balance,
		) -> DispatchResult;

		fn swap(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: Self::Balance,
			min_dy: Self::Balance,
			asset_length: u32,
		) -> DispatchResult;

		fn redeem_proportion(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			min_redeem_amounts: Vec<Self::Balance>,
		) -> DispatchResult;

		fn redeem_single(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: Self::Balance,
			asset_length: u32,
		) -> DispatchResult;

		fn redeem_multi(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amounts: Vec<Self::Balance>,
			max_redeem_amount: Self::Balance,
		) -> DispatchResult;

		fn collect_fee(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn update_balance(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn collect_yield(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn modify_a(
			pool_id: StableAssetPoolId,
			a: Self::AtLeast64BitUnsigned,
			future_a_block: Self::BlockNumber,
		) -> DispatchResult;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::{PoolTokenIndex, StableAssetPoolId, StableAssetPoolInfo};
	use crate::traits::{StableAsset, ValidateAssetId};
	use crate::WeightInfo;
	use frame_support::traits::tokens::fungibles;
	use frame_support::{
		dispatch::{Codec, DispatchResult},
		pallet_prelude::*,
		traits::EnsureOrigin,
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero};
	use sp_std::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type AssetId: Parameter + Ord + Copy;
		type Balance: Parameter + Codec + Copy + Ord + From<Self::AtLeast64BitUnsigned> + Zero;
		type Assets: fungibles::Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ fungibles::Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ fungibles::Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;
		type AtLeast64BitUnsigned: Parameter
			+ CheckedAdd
			+ CheckedSub
			+ CheckedMul
			+ CheckedDiv
			+ Copy
			+ Eq
			+ Ord
			+ From<Self::Balance>
			+ From<u8>
			+ From<u128>
			+ From<Self::BlockNumber>
			+ TryFrom<usize>
			+ Zero
			+ One;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type FeePrecision: Get<Self::AtLeast64BitUnsigned>;
		#[pallet::constant]
		type APrecision: Get<Self::AtLeast64BitUnsigned>;
		#[pallet::constant]
		type PoolAssetLimit: Get<u32>;
		type WeightInfo: WeightInfo;
		type EnsurePoolAssetId: ValidateAssetId<Self::AssetId>;

		/// The origin which may create pool or modify pool.
		type ListingOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn pool_count)]
	pub type PoolCount<T: Config> = StorageValue<_, StableAssetPoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StableAssetPoolId,
		StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreatePool {
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			swap_id: T::AccountId,
			pallet_id: T::AccountId,
		},
		Minted {
			minter: T::AccountId,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			input_amounts: Vec<T::Balance>,
			min_output_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			fee_amount: T::Balance,
			output_amount: T::Balance,
		},
		TokenSwapped {
			swapper: T::AccountId,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			input_asset: T::AssetId,
			output_asset: T::AssetId,
			input_amount: T::Balance,
			min_output_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			output_amount: T::Balance,
		},
		RedeemedProportion {
			redeemer: T::AccountId,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			input_amount: T::Balance,
			min_output_amounts: Vec<T::Balance>,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			fee_amount: T::Balance,
			output_amounts: Vec<T::Balance>,
		},
		RedeemedSingle {
			redeemer: T::AccountId,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			input_amount: T::Balance,
			output_asset: T::AssetId,
			min_output_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			fee_amount: T::Balance,
			output_amount: T::Balance,
		},
		RedeemedMulti {
			redeemer: T::AccountId,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			output_amounts: Vec<T::Balance>,
			max_input_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			fee_amount: T::Balance,
			input_amount: T::Balance,
		},
		BalanceUpdated {
			pool_id: StableAssetPoolId,
			old_balances: Vec<T::Balance>,
			new_balances: Vec<T::Balance>,
		},
		YieldCollected {
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			old_total_supply: T::Balance,
			new_total_supply: T::Balance,
			who: T::AccountId,
			amount: T::Balance,
		},
		FeeCollected {
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			old_balances: Vec<T::Balance>,
			new_balances: Vec<T::Balance>,
			old_total_supply: T::Balance,
			new_total_supply: T::Balance,
			who: T::AccountId,
			amount: T::Balance,
		},
		AModified {
			pool_id: StableAssetPoolId,
			value: T::AtLeast64BitUnsigned,
			time: T::BlockNumber,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		InconsistentStorage,
		InvalidPoolAsset,
		ArgumentsMismatch,
		ArgumentsError,
		PoolNotFound,
		Math,
		InvalidPoolValue,
		MintUnderMin,
		SwapUnderMin,
		RedeemUnderMin,
		RedeemOverMax,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct MintResult<T: Config> {
		pub mint_amount: T::Balance,
		pub fee_amount: T::Balance,
		pub balances: Vec<T::Balance>,
		pub total_supply: T::Balance,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct SwapResult<T: Config> {
		pub dy: T::Balance,
		pub y: T::Balance,
		pub balance_i: T::Balance,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemProportionResult<T: Config> {
		pub amounts: Vec<T::Balance>,
		pub balances: Vec<T::Balance>,
		pub fee_amount: T::Balance,
		pub total_supply: T::Balance,
		pub redeem_amount: T::Balance,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemSingleResult<T: Config> {
		pub dy: T::Balance,
		pub fee_amount: T::Balance,
		pub total_supply: T::Balance,
		pub balances: Vec<T::Balance>,
		pub redeem_amount: T::Balance,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemMultiResult<T: Config> {
		pub redeem_amount: T::Balance,
		pub fee_amount: T::Balance,
		pub balances: Vec<T::Balance>,
		pub total_supply: T::Balance,
		pub burn_amount: T::Balance,
	}

	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct PendingFeeResult<T: Config> {
		pub fee_amount: T::Balance,
		pub balances: Vec<T::Balance>,
		pub total_supply: T::Balance,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_asset: T::AssetId,
			assets: Vec<T::AssetId>,
			precisions: Vec<T::AtLeast64BitUnsigned>,
			mint_fee: T::AtLeast64BitUnsigned,
			swap_fee: T::AtLeast64BitUnsigned,
			redeem_fee: T::AtLeast64BitUnsigned,
			initial_a: T::AtLeast64BitUnsigned,
			fee_recipient: T::AccountId,
			yield_recipient: T::AccountId,
			precision: T::AtLeast64BitUnsigned,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			ensure!(T::EnsurePoolAssetId::validate(pool_asset), Error::<T>::InvalidPoolAsset);
			<Self as StableAsset>::create_pool(
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
			)
		}

		#[pallet::weight(T::WeightInfo::mint(amounts.len() as u32))]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			min_mint_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::mint(&who, pool_id, amounts, min_mint_amount)
		}

		#[pallet::weight(T::WeightInfo::swap(*asset_length))]
		#[transactional]
		pub fn swap(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: T::Balance,
			min_dy: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::swap(&who, pool_id, i, j, dx, min_dy, asset_length)
		}

		#[pallet::weight(T::WeightInfo::redeem_proportion(min_redeem_amounts.len() as u32))]
		#[transactional]
		pub fn redeem_proportion(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_proportion(&who, pool_id, amount, min_redeem_amounts)
		}

		#[pallet::weight(T::WeightInfo::redeem_single(*asset_length))]
		#[transactional]
		pub fn redeem_single(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_single(&who, pool_id, amount, i, min_redeem_amount, asset_length)
		}

		#[pallet::weight(T::WeightInfo::redeem_multi(amounts.len() as u32))]
		#[transactional]
		pub fn redeem_multi(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			max_redeem_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_multi(&who, pool_id, amounts, max_redeem_amount)
		}

		#[pallet::weight(T::WeightInfo::modify_a())]
		#[transactional]
		pub fn modify_a(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			future_a_block: T::BlockNumber,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			<Self as StableAsset>::modify_a(pool_id, a, future_a_block)
		}
	}
}
impl<T: Config> Pallet<T> {
	pub(crate) fn convert_vec_number_to_balance(numbers: Vec<T::AtLeast64BitUnsigned>) -> Vec<T::Balance> {
		numbers.into_iter().map(|x| x.into()).collect()
	}

	pub(crate) fn convert_vec_balance_to_number(balances: Vec<T::Balance>) -> Vec<T::AtLeast64BitUnsigned> {
		balances.into_iter().map(|x| x.into()).collect()
	}

	pub(crate) fn get_a(
		a0: T::AtLeast64BitUnsigned,
		t0: T::BlockNumber,
		a1: T::AtLeast64BitUnsigned,
		t1: T::BlockNumber,
	) -> Option<T::AtLeast64BitUnsigned> {
		let current_block = frame_system::Pallet::<T>::block_number();
		if current_block < t1 {
			let time_diff: T::AtLeast64BitUnsigned = current_block.checked_sub(&t0)?.into();
			let time_diff_div: T::AtLeast64BitUnsigned = t1.checked_sub(&t0)?.into();
			if a1 > a0 {
				let diff = a1.checked_sub(&a0)?;
				let amount = diff.checked_mul(&time_diff)?.checked_div(&time_diff_div)?;
				Some(a0.checked_add(&amount)?)
			} else {
				let diff = a0.checked_sub(&a1)?;
				let amount = diff.checked_mul(&time_diff)?.checked_div(&time_diff_div)?;
				Some(a0.checked_sub(&amount)?)
			}
		} else {
			Some(a1)
		}
	}

	pub(crate) fn get_d(
		balances: &[T::AtLeast64BitUnsigned],
		a: T::AtLeast64BitUnsigned,
	) -> Option<T::AtLeast64BitUnsigned> {
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let mut sum: T::AtLeast64BitUnsigned = zero;
		let mut ann: T::AtLeast64BitUnsigned = a;
		let balance_size: T::AtLeast64BitUnsigned = T::AtLeast64BitUnsigned::try_from(balances.len()).ok()?;
		for x in balances.iter() {
			sum = sum.checked_add(x)?;
			ann = ann.checked_mul(&balance_size)?;
		}
		if sum == zero {
			return Some(zero);
		}

		let mut prev_d: T::AtLeast64BitUnsigned;
		let mut d: T::AtLeast64BitUnsigned = sum;

		for _i in 0..NUMBER_OF_ITERATIONS_TO_CONVERGE {
			let mut p_d: T::AtLeast64BitUnsigned = d;
			for x in balances.iter() {
				let div_op = x.checked_mul(&balance_size)?;
				p_d = p_d.checked_mul(&d)?.checked_div(&div_op)?;
			}
			prev_d = d;
			let t1: T::AtLeast64BitUnsigned = p_d.checked_mul(&balance_size)?;
			let t2: T::AtLeast64BitUnsigned = balance_size.checked_add(&one)?.checked_mul(&p_d)?;
			let t3: T::AtLeast64BitUnsigned = ann
				.checked_sub(&T::APrecision::get())?
				.checked_mul(&d)?
				.checked_div(&T::APrecision::get())?
				.checked_add(&t2)?;
			d = ann
				.checked_mul(&sum)?
				.checked_div(&T::APrecision::get())?
				.checked_add(&t1)?
				.checked_mul(&d)?
				.checked_div(&t3)?;
			if d > prev_d {
				if d - prev_d <= one {
					break;
				}
			} else if prev_d - d <= one {
				break;
			}
		}
		Some(d)
	}

	pub(crate) fn get_y(
		balances: &[T::AtLeast64BitUnsigned],
		token_index: PoolTokenIndex,
		target_d: T::AtLeast64BitUnsigned,
		amplitude: T::AtLeast64BitUnsigned,
	) -> Option<T::AtLeast64BitUnsigned> {
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let two: T::AtLeast64BitUnsigned = 2u8.into();
		let mut c: T::AtLeast64BitUnsigned = target_d;
		let mut sum: T::AtLeast64BitUnsigned = zero;
		let mut ann: T::AtLeast64BitUnsigned = amplitude;
		let balance_size: T::AtLeast64BitUnsigned = T::AtLeast64BitUnsigned::try_from(balances.len()).ok()?;

		for (i, balance_ref) in balances.iter().enumerate() {
			let balance: T::AtLeast64BitUnsigned = *balance_ref;
			ann = ann.checked_mul(&balance_size)?;
			let token_index_usize = token_index as usize;
			if i == token_index_usize {
				continue;
			}
			sum = sum.checked_add(&balance)?;
			let div_op = balance.checked_mul(&balance_size)?;
			c = c.checked_mul(&target_d)?.checked_div(&div_op)?
		}

		c = c
			.checked_mul(&target_d)?
			.checked_mul(&T::APrecision::get())?
			.checked_div(&ann.checked_mul(&balance_size)?)?;
		let b: T::AtLeast64BitUnsigned =
			sum.checked_add(&target_d.checked_mul(&T::APrecision::get())?.checked_div(&ann)?)?;
		let mut prev_y: T::AtLeast64BitUnsigned;
		let mut y: T::AtLeast64BitUnsigned = target_d;

		for _i in 0..NUMBER_OF_ITERATIONS_TO_CONVERGE {
			prev_y = y;
			y = y
				.checked_mul(&y)?
				.checked_add(&c)?
				.checked_div(&y.checked_mul(&two)?.checked_add(&b)?.checked_sub(&target_d)?)?;
			if y > prev_y {
				if y - prev_y <= one {
					break;
				}
			} else if prev_y - y <= one {
				break;
			}
		}
		Some(y)
	}

	pub(crate) fn get_mint_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
		amounts_bal: &[T::Balance],
	) -> Result<MintResult<T>, Error<T>> {
		if pool_info.balances.len() != amounts_bal.len() {
			return Err(Error::<T>::ArgumentsMismatch);
		}
		let amounts = Self::convert_vec_balance_to_number(amounts_bal.to_vec());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		for i in 0..balances.len() {
			if amounts[i] == zero {
				if old_d == zero {
					return Err(Error::<T>::ArgumentsError);
				}
				continue;
			}
			let result: T::AtLeast64BitUnsigned = balances[i]
				.checked_add(
					&amounts[i]
						.checked_mul(&pool_info.precisions[i])
						.ok_or(Error::<T>::Math)?,
				)
				.ok_or(Error::<T>::Math)?;
			balances[i] = result;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let mut mint_amount: T::AtLeast64BitUnsigned = new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		let mint_fee: T::AtLeast64BitUnsigned = pool_info.mint_fee;

		if pool_info.mint_fee > zero {
			fee_amount = mint_amount
				.checked_mul(&mint_fee)
				.ok_or(Error::<T>::Math)?
				.checked_div(&fee_denominator)
				.ok_or(Error::<T>::Math)?;
			mint_amount = mint_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		Ok(MintResult {
			mint_amount: mint_amount.into(),
			fee_amount: fee_amount.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			total_supply: new_d.into(),
		})
	}

	pub(crate) fn get_swap_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
		input_index: PoolTokenIndex,
		output_index: PoolTokenIndex,
		dx_bal: T::Balance,
	) -> Result<SwapResult<T>, Error<T>> {
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let balance_size: usize = pool_info.balances.len();
		let dx: T::AtLeast64BitUnsigned = dx_bal.into();
		let input_index_usize = input_index as usize;
		let output_index_usize = output_index as usize;
		if input_index == output_index {
			return Err(Error::<T>::ArgumentsError);
		}
		if dx <= zero {
			return Err(Error::<T>::ArgumentsError);
		}
		if input_index_usize >= balance_size {
			return Err(Error::<T>::ArgumentsError);
		}
		if output_index_usize >= balance_size {
			return Err(Error::<T>::ArgumentsError);
		}

		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		balances[input_index_usize] = balances[input_index_usize]
			.checked_add(
				&dx.checked_mul(&pool_info.precisions[input_index_usize])
					.ok_or(Error::<T>::Math)?,
			)
			.ok_or(Error::<T>::Math)?;
		let y: T::AtLeast64BitUnsigned = Self::get_y(&balances, output_index, d, a).ok_or(Error::<T>::Math)?;
		let mut dy: T::AtLeast64BitUnsigned = balances[output_index_usize]
			.checked_sub(&y)
			.ok_or(Error::<T>::Math)?
			.checked_sub(&one)
			.ok_or(Error::<T>::Math)?
			.checked_div(&pool_info.precisions[output_index_usize])
			.ok_or(Error::<T>::Math)?;
		if pool_info.swap_fee > zero {
			let fee_amount: T::AtLeast64BitUnsigned = dy
				.checked_mul(&pool_info.swap_fee)
				.ok_or(Error::<T>::Math)?
				.checked_div(&fee_denominator)
				.ok_or(Error::<T>::Math)?;
			dy = dy.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}
		Ok(SwapResult {
			dy: dy.into(),
			y: y.into(),
			balance_i: balances[input_index_usize].into(),
		})
	}

	pub(crate) fn get_redeem_proportion_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
		amount_bal: T::Balance,
	) -> Result<RedeemProportionResult<T>, Error<T>> {
		let mut amount: T::AtLeast64BitUnsigned = amount_bal.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();

		if amount <= zero {
			return Err(Error::<T>::ArgumentsError);
		}

		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let mut amounts: Vec<T::AtLeast64BitUnsigned> = Vec::new();
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		if pool_info.redeem_fee > zero {
			fee_amount = amount
				.checked_mul(&pool_info.redeem_fee)
				.ok_or(Error::<T>::Math)?
				.checked_div(&fee_denominator)
				.ok_or(Error::<T>::Math)?;
			// Redemption fee is charged with pool token before redemption.
			amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		for i in 0..pool_info.balances.len() {
			let balance_i: T::AtLeast64BitUnsigned = balances[i];
			let diff_i: T::AtLeast64BitUnsigned = balance_i
				.checked_mul(&amount)
				.ok_or(Error::<T>::Math)?
				.checked_div(&d)
				.ok_or(Error::<T>::Math)?;
			balances[i] = balance_i.checked_sub(&diff_i).ok_or(Error::<T>::Math)?;
			let amounts_i: T::AtLeast64BitUnsigned =
				diff_i.checked_div(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
			amounts.push(amounts_i);
		}
		let total_supply: T::AtLeast64BitUnsigned = d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
		Ok(RedeemProportionResult {
			amounts: Self::convert_vec_number_to_balance(amounts),
			balances: Self::convert_vec_number_to_balance(balances),
			fee_amount: fee_amount.into(),
			total_supply: total_supply.into(),
			redeem_amount: amount.into(),
		})
	}

	pub(crate) fn get_redeem_single_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
		amount_bal: T::Balance,
		i: PoolTokenIndex,
	) -> Result<RedeemSingleResult<T>, Error<T>> {
		let mut amount: T::AtLeast64BitUnsigned = amount_bal.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let i_usize = i as usize;
		if amount <= zero {
			return Err(Error::<T>::ArgumentsError);
		}
		if i_usize >= pool_info.balances.len() {
			return Err(Error::<T>::ArgumentsError);
		}
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;

		if pool_info.redeem_fee > zero {
			fee_amount = amount
				.checked_mul(&pool_info.redeem_fee)
				.ok_or(Error::<T>::Math)?
				.checked_div(&fee_denominator)
				.ok_or(Error::<T>::Math)?;
			// Redemption fee is charged with pool token before redemption.
			amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		// The pool token amount becomes D - _amount
		let y: T::AtLeast64BitUnsigned =
			Self::get_y(&balances, i, d.checked_sub(&amount).ok_or(Error::<T>::Math)?, a).ok_or(Error::<T>::Math)?;
		// dy = (balance[i] - y - 1) / precisions[i] in case there was rounding errors
		let balance_i: T::AtLeast64BitUnsigned = pool_info.balances[i_usize].into();
		let dy: T::AtLeast64BitUnsigned = balance_i
			.checked_sub(&y)
			.ok_or(Error::<T>::Math)?
			.checked_sub(&one)
			.ok_or(Error::<T>::Math)?
			.checked_div(&pool_info.precisions[i_usize])
			.ok_or(Error::<T>::Math)?;
		let total_supply: T::AtLeast64BitUnsigned = d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
		balances[i_usize] = y;
		Ok(RedeemSingleResult {
			dy: dy.into(),
			fee_amount: fee_amount.into(),
			total_supply: total_supply.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			redeem_amount: amount.into(),
		})
	}

	pub(crate) fn get_redeem_multi_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
		amounts: &[T::Balance],
	) -> Result<RedeemMultiResult<T>, Error<T>> {
		if amounts.len() != pool_info.balances.len() {
			return Err(Error::<T>::ArgumentsError);
		}
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		for i in 0..balances.len() {
			let amounts_i: T::AtLeast64BitUnsigned = amounts[i].into();
			if amounts_i == zero {
				continue;
			}
			let balance_i: T::AtLeast64BitUnsigned = balances[i];
			// balance = balance + amount * precision
			let sub_amount: T::AtLeast64BitUnsigned = amounts_i
				.checked_mul(&pool_info.precisions[i])
				.ok_or(Error::<T>::Math)?;
			balances[i] = balance_i.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let mut redeem_amount: T::AtLeast64BitUnsigned = old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		if pool_info.redeem_fee > zero {
			let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
			let div_amount: T::AtLeast64BitUnsigned = fee_denominator
				.checked_sub(&pool_info.redeem_fee)
				.ok_or(Error::<T>::Math)?;
			redeem_amount = redeem_amount
				.checked_mul(&fee_denominator)
				.ok_or(Error::<T>::Math)?
				.checked_div(&div_amount)
				.ok_or(Error::<T>::Math)?;
			let sub_amount: T::AtLeast64BitUnsigned = old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
			fee_amount = redeem_amount.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
		}
		let burn_amount: T::AtLeast64BitUnsigned = redeem_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		let total_supply: T::AtLeast64BitUnsigned = old_d.checked_sub(&burn_amount).ok_or(Error::<T>::Math)?;
		Ok(RedeemMultiResult {
			redeem_amount: redeem_amount.into(),
			fee_amount: fee_amount.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			total_supply: total_supply.into(),
			burn_amount: burn_amount.into(),
		})
	}

	pub(crate) fn get_pending_fee_amount(
		pool_info: &StableAssetPoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId, T::BlockNumber>,
	) -> Result<PendingFeeResult<T>, Error<T>> {
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		for (i, balance) in balances.iter_mut().enumerate() {
			let balance_of: T::AtLeast64BitUnsigned =
				T::Assets::balance(pool_info.assets[i], &pool_info.account_id).into();
			*balance = balance_of
				.checked_mul(&pool_info.precisions[i])
				.ok_or(Error::<T>::Math)?;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;

		if new_d > old_d {
			let fee_amount: T::AtLeast64BitUnsigned = new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;
			Ok(PendingFeeResult {
				fee_amount: fee_amount.into(),
				balances: Self::convert_vec_number_to_balance(balances),
				total_supply: new_d.into(),
			})
		} else {
			// this is due to rounding issues for token balance conversion
			Ok(PendingFeeResult {
				fee_amount: Zero::zero(),
				balances: Self::convert_vec_number_to_balance(balances),
				total_supply: new_d.into(),
			})
		}
	}
}

impl<T: Config> StableAsset for Pallet<T> {
	type AssetId = T::AssetId;
	type AtLeast64BitUnsigned = T::AtLeast64BitUnsigned;
	type Balance = T::Balance;
	type AccountId = T::AccountId;
	type BlockNumber = T::BlockNumber;

	fn pool_count() -> StableAssetPoolId {
		PoolCount::<T>::get()
	}

	fn pool(
		id: StableAssetPoolId,
	) -> Option<
		StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	> {
		Pools::<T>::get(id)
	}

	fn update_balance(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let old_balances = pool_info.balances.clone();
		for (i, balance) in pool_info.balances.iter_mut().enumerate() {
			let balance_of: Self::AtLeast64BitUnsigned =
				T::Assets::balance(pool_info.assets[i], &pool_info.account_id).into();
			*balance = balance_of
				.checked_mul(&pool_info.precisions[i])
				.ok_or(Error::<T>::Math)?
				.into();
		}
		Self::deposit_event(Event::BalanceUpdated {
			pool_id,
			old_balances,
			new_balances: pool_info.balances.clone(),
		});
		Ok(())
	}

	fn collect_yield(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_total_supply = pool_info.total_supply;
		let old_d: T::AtLeast64BitUnsigned = old_total_supply.into();
		Self::update_balance(pool_id, pool_info)?;
		let balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;

		ensure!(new_d >= old_d, Error::<T>::InvalidPoolValue);
		if new_d > old_d {
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			let yield_amount = new_d - old_d;
			T::Assets::mint_into(pool_info.pool_asset, &pool_info.yield_recipient, yield_amount.into())?;
			pool_info.total_supply = new_d.into();
			Self::deposit_event(Event::YieldCollected {
				pool_id,
				a,
				old_total_supply,
				new_total_supply: pool_info.total_supply,
				who: pool_info.yield_recipient.clone(),
				amount: yield_amount.into(),
			});
		}
		Ok(())
	}

	fn collect_fee(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let old_balances = pool_info.balances.clone();
		let old_total_supply = pool_info.total_supply;
		let PendingFeeResult {
			fee_amount,
			balances,
			total_supply,
		} = Self::get_pending_fee_amount(pool_info)?;
		let zero: T::Balance = Zero::zero();
		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		if fee_amount > zero {
			let fee_recipient = pool_info.fee_recipient.clone();
			T::Assets::mint_into(pool_info.pool_asset, &fee_recipient, fee_amount)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::FeeCollected {
				pool_id,
				a,
				old_balances,
				new_balances: pool_info.balances.clone(),
				old_total_supply,
				new_total_supply: total_supply,
				who: fee_recipient,
				amount: fee_amount,
			});
		}
		Ok(())
	}

	fn create_pool(
		pool_asset: Self::AssetId,
		assets: Vec<Self::AssetId>,
		precisions: Vec<Self::AtLeast64BitUnsigned>,
		mint_fee: Self::AtLeast64BitUnsigned,
		swap_fee: Self::AtLeast64BitUnsigned,
		redeem_fee: Self::AtLeast64BitUnsigned,
		initial_a: Self::AtLeast64BitUnsigned,
		fee_recipient: Self::AccountId,
		yield_recipient: Self::AccountId,
		precision: Self::AtLeast64BitUnsigned,
	) -> DispatchResult {
		ensure!(assets.len() > 1, Error::<T>::ArgumentsError);
		let pool_asset_limit = T::PoolAssetLimit::get() as usize;
		ensure!(assets.len() <= pool_asset_limit, Error::<T>::ArgumentsError);
		ensure!(assets.len() == precisions.len(), Error::<T>::ArgumentsMismatch);
		PoolCount::<T>::try_mutate(|pool_count| -> DispatchResult {
			let pool_id = *pool_count;
			let swap_id: T::AccountId = T::PalletId::get().into_sub_account(pool_id);
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				ensure!(maybe_pool_info.is_none(), Error::<T>::InconsistentStorage);

				let balances = sp_std::vec![Zero::zero(); assets.len()];
				frame_system::Pallet::<T>::inc_providers(&swap_id);
				let current_block = frame_system::Pallet::<T>::block_number();
				*maybe_pool_info = Some(StableAssetPoolInfo {
					pool_asset,
					assets,
					precisions,
					mint_fee,
					swap_fee,
					redeem_fee,
					total_supply: Zero::zero(),
					a: initial_a,
					a_block: current_block,
					future_a: initial_a,
					future_a_block: current_block,
					balances,
					fee_recipient,
					account_id: swap_id.clone(),
					yield_recipient,
					precision,
				});

				Ok(())
			})?;

			*pool_count = pool_id.checked_add(1).ok_or(Error::<T>::InconsistentStorage)?;

			Self::deposit_event(Event::CreatePool {
				pool_id,
				swap_id,
				a: initial_a,
				pallet_id: T::PalletId::get().into_account(),
			});
			Ok(())
		})
	}

	fn mint(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		min_mint_amount: Self::Balance,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			let MintResult {
				mint_amount,
				fee_amount,
				balances,
				total_supply,
			} = Self::get_mint_amount(pool_info, &amounts)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			ensure!(mint_amount >= min_mint_amount, Error::<T>::MintUnderMin);
			for (i, amount) in amounts.iter().enumerate() {
				if *amount == Zero::zero() {
					continue;
				}
				T::Assets::transfer(pool_info.assets[i], who, &pool_info.account_id, *amount, false)?;
			}

			let zero: T::Balance = Zero::zero();
			if fee_amount > zero {
				T::Assets::mint_into(pool_info.pool_asset, &pool_info.fee_recipient, fee_amount)?;
			}
			T::Assets::mint_into(pool_info.pool_asset, who, mint_amount)?;
			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			Self::collect_fee(pool_id, pool_info)?;
			Self::deposit_event(Event::Minted {
				minter: who.clone(),
				pool_id,
				a,
				input_amounts: amounts,
				min_output_amount: min_mint_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amount: mint_amount,
			});
			Ok(())
		})
	}

	fn swap(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		i: PoolTokenIndex,
		j: PoolTokenIndex,
		dx: Self::Balance,
		min_dy: Self::Balance,
		asset_length: u32,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			let asset_length_usize = asset_length as usize;
			ensure!(asset_length_usize == pool_info.assets.len(), Error::<T>::ArgumentsError);
			Self::collect_yield(pool_id, pool_info)?;
			let SwapResult { dy, y, balance_i } = Self::get_swap_amount(pool_info, i, j, dx)?;
			ensure!(dy >= min_dy, Error::<T>::SwapUnderMin);
			let mut balances = pool_info.balances.clone();
			let i_usize = i as usize;
			let j_usize = j as usize;
			balances[i_usize] = balance_i;
			balances[j_usize] = y;
			T::Assets::transfer(pool_info.assets[i_usize], who, &pool_info.account_id, dx, false)?;
			T::Assets::transfer(pool_info.assets[j_usize], &pool_info.account_id, who, dy, false)?;
			let asset_i = pool_info.assets[i_usize];
			let asset_j = pool_info.assets[j_usize];

			// Since the actual output amount is round down, collect fee should update the pool balances and
			// total supply
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::TokenSwapped {
				swapper: who.clone(),
				pool_id,
				a,
				input_asset: asset_i,
				output_asset: asset_j,
				input_amount: dx,
				min_output_amount: min_dy,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				output_amount: dy,
			});
			Ok(())
		})
	}

	fn redeem_proportion(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			ensure!(
				min_redeem_amounts.len() == pool_info.assets.len(),
				Error::<T>::ArgumentsMismatch
			);
			let RedeemProportionResult {
				amounts,
				balances,
				fee_amount,
				total_supply,
				redeem_amount,
			} = Self::get_redeem_proportion_amount(pool_info, amount)?;
			let zero: T::Balance = Zero::zero();
			for i in 0..amounts.len() {
				ensure!(amounts[i] >= min_redeem_amounts[i], Error::<T>::RedeemUnderMin);
				T::Assets::transfer(pool_info.assets[i], &pool_info.account_id, who, amounts[i], false)?;
			}
			if fee_amount > zero {
				T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, false)?;
			}
			T::Assets::burn_from(pool_info.pool_asset, who, redeem_amount)?;

			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			// Since the output amounts are round down, collect fee updates pool balances and total supply.
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::RedeemedProportion {
				redeemer: who.clone(),
				pool_id,
				a,
				input_amount: amount,
				min_output_amounts: min_redeem_amounts,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amounts: amounts,
			});
			Ok(())
		})
	}

	fn redeem_single(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		i: PoolTokenIndex,
		min_redeem_amount: Self::Balance,
		asset_length: u32,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			let RedeemSingleResult {
				dy,
				fee_amount,
				total_supply,
				balances,
				redeem_amount,
			} = Self::get_redeem_single_amount(pool_info, amount, i)?;
			let i_usize = i as usize;
			let pool_size = pool_info.assets.len();
			let asset_length_usize = asset_length as usize;
			ensure!(asset_length_usize == pool_size, Error::<T>::ArgumentsError);
			ensure!(dy >= min_redeem_amount, Error::<T>::RedeemUnderMin);
			if fee_amount > Zero::zero() {
				T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, false)?;
			}
			T::Assets::transfer(pool_info.assets[i_usize], &pool_info.account_id, who, dy, false)?;
			T::Assets::burn_from(pool_info.pool_asset, who, redeem_amount)?;
			let mut amounts: Vec<T::Balance> = Vec::new();
			for idx in 0..pool_size {
				if idx == i_usize {
					amounts.push(dy);
				} else {
					amounts.push(Zero::zero());
				}
			}

			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			// Since the output amounts are round down, collect fee updates pool balances and total supply.
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::RedeemedSingle {
				redeemer: who.clone(),
				pool_id,
				a,
				input_amount: amount,
				output_asset: pool_info.assets[i as usize],
				min_output_amount: min_redeem_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amount: dy,
			});
			Ok(())
		})
	}

	fn redeem_multi(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		max_redeem_amount: Self::Balance,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			let RedeemMultiResult {
				redeem_amount,
				fee_amount,
				balances,
				total_supply,
				burn_amount,
			} = Self::get_redeem_multi_amount(pool_info, &amounts)?;
			let zero: T::Balance = Zero::zero();
			ensure!(redeem_amount <= max_redeem_amount, Error::<T>::RedeemOverMax);
			if fee_amount > zero {
				T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, false)?;
			}
			for (idx, amount) in amounts.iter().enumerate() {
				if *amount > zero {
					T::Assets::transfer(pool_info.assets[idx], &pool_info.account_id, who, amounts[idx], false)?;
				}
			}
			T::Assets::burn_from(pool_info.pool_asset, who, burn_amount)?;

			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::RedeemedMulti {
				redeemer: who.clone(),
				pool_id,
				a,
				output_amounts: amounts,
				max_input_amount: max_redeem_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				input_amount: redeem_amount,
			});
			Ok(())
		})
	}

	fn modify_a(
		pool_id: StableAssetPoolId,
		a: Self::AtLeast64BitUnsigned,
		future_a_block: T::BlockNumber,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			ensure!(future_a_block > pool_info.a_block, Error::<T>::ArgumentsError);
			let current_block = frame_system::Pallet::<T>::block_number();
			let initial_a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			pool_info.a = initial_a;
			pool_info.a_block = current_block;
			pool_info.future_a = a;
			pool_info.future_a_block = future_a_block;
			Self::deposit_event(Event::AModified {
				pool_id,
				value: a,
				time: future_a_block,
			});
			Ok(())
		})
	}
}
