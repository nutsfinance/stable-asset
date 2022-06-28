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

use crate::traits::{StableAssetXcm, XcmInterface};
use frame_support::codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use frame_support::ensure;
use frame_support::traits::fungibles::Mutate;
use frame_support::weights::Weight;
use scale_info::TypeInfo;

use sp_runtime::traits::Zero;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

pub type ParachainId = u32;

pub type StableAssetXcmPoolId = u32;

pub type PoolTokenIndex = u32;

pub type StableAssetPoolId = u32;

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub struct StableAssetXcmPoolInfo<AssetId, Balance> {
	pub pool_asset: AssetId,
	pub balances: BTreeMap<ParachainId, Balance>,
	pub limits: BTreeMap<ParachainId, Balance>,
	pub remote_stable_assets: BTreeMap<ParachainId, StableAssetPoolId>,
}

pub trait WeightInfo {
	fn create_pool() -> Weight;
	fn update_limit() -> Weight;
	fn update_remote_stable_asset() -> Weight;
	fn mint() -> Weight;
	fn redeem_proportion() -> Weight;
	fn redeem_single() -> Weight;
}

pub mod traits {
	use crate::{ParachainId, PoolTokenIndex, StableAssetPoolId, StableAssetXcmPoolId, StableAssetXcmPoolInfo};
	use frame_support::dispatch::DispatchResult;
	use sp_std::prelude::*;

	pub trait ValidateAssetId<AssetId> {
		fn validate(a: AssetId) -> bool;
	}

	pub trait StableAssetXcm {
		type AssetId;
		type Balance;
		type AccountId;

		fn pool_count() -> StableAssetXcmPoolId;

		fn pool(id: StableAssetXcmPoolId) -> Option<StableAssetXcmPoolInfo<Self::AssetId, Self::Balance>>;

		fn create_pool(pool_asset: Self::AssetId) -> DispatchResult;

		fn update_limit(pool_id: StableAssetXcmPoolId, chain_id: ParachainId, limit: Self::Balance) -> DispatchResult;

		fn update_remote_stable_asset(
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			remote_pool_id: StableAssetPoolId,
		) -> DispatchResult;

		fn mint(
			who: Self::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: Self::Balance,
		) -> DispatchResult;

		fn redeem_proportion(
			who: &Self::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: Self::Balance,
			min_redeem_amounts: Vec<Self::Balance>,
		) -> DispatchResult;

		fn redeem_single(
			who: &Self::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: Self::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: Self::Balance,
			asset_length: u32,
		) -> DispatchResult;
	}

	pub trait XcmInterface {
		type Balance;
		type AccountId;
		fn send_mint_failed(
			account_id: Self::AccountId,
			chain_id: ParachainId,
			pool_id: StableAssetPoolId,
			mint_amount: Self::Balance,
		) -> DispatchResult;

		fn send_redeem_proportion(
			account_id: Self::AccountId,
			chain_id: ParachainId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			min_redeem_amounts: Vec<Self::Balance>,
		) -> DispatchResult;

		fn send_redeem_single(
			account_id: Self::AccountId,
			chain_id: ParachainId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: Self::Balance,
			asset_length: u32,
		) -> DispatchResult;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::{ParachainId, PoolTokenIndex, StableAssetPoolId, StableAssetXcmPoolId, StableAssetXcmPoolInfo};
	use crate::traits::{StableAssetXcm, ValidateAssetId, XcmInterface};
	use crate::WeightInfo;
	use frame_support::traits::tokens::fungibles;
	use frame_support::{
		dispatch::{Codec, DispatchResult},
		pallet_prelude::*,
		traits::EnsureOrigin,
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Zero;
	use sp_std::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type AssetId: Parameter + Ord + Copy;
		type Balance: Parameter + Codec + Copy + Ord + Zero;
		type Assets: fungibles::Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ fungibles::Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ fungibles::Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type WeightInfo: WeightInfo;
		type EnsurePoolAssetId: ValidateAssetId<Self::AssetId>;
		type XcmInterface: XcmInterface<Balance = Self::Balance, AccountId = Self::AccountId>;

		/// The origin which may create pool or modify pool.
		type ListingOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn pool_count)]
	pub type PoolCount<T: Config> = StorageValue<_, StableAssetXcmPoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> =
		StorageMap<_, Blake2_128Concat, StableAssetXcmPoolId, StableAssetXcmPoolInfo<T::AssetId, T::Balance>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreatePool {
			pool_id: StableAssetXcmPoolId,
		},
		LimitUpdated {
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			limit: T::Balance,
		},
		RemotePoolUpdated {
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			remote_pool_id: StableAssetPoolId,
		},
		Minted {
			minter: T::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			mint_amount: T::Balance,
		},
		RedeemedProportion {
			redeemer: T::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			input_amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		},
		RedeemedSingle {
			redeemer: T::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			input_amount: T::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: T::Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		InconsistentStorage,
		InvalidPoolAsset,
		NewLimitInvalid,
		PoolNotFound,
		RemotePoolNotFound,
		RedeemOverLimit,
		MintOverLimit,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(origin: OriginFor<T>, pool_asset: T::AssetId) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			ensure!(T::EnsurePoolAssetId::validate(pool_asset), Error::<T>::InvalidPoolAsset);
			<Self as StableAssetXcm>::create_pool(pool_asset)
		}

		#[pallet::weight(T::WeightInfo::update_limit())]
		#[transactional]
		pub fn update_limit(
			origin: OriginFor<T>,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			limit: T::Balance,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			<Self as StableAssetXcm>::update_limit(pool_id, chain_id, limit)
		}

		#[pallet::weight(T::WeightInfo::update_remote_stable_asset())]
		#[transactional]
		pub fn update_remote_stable_asset(
			origin: OriginFor<T>,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			remote_pool_id: StableAssetPoolId,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			<Self as StableAssetXcm>::update_remote_stable_asset(pool_id, chain_id, remote_pool_id)
		}

		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			account_id: T::AccountId,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: T::Balance,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin)?;
			<Self as StableAssetXcm>::mint(account_id, pool_id, chain_id, amount)
		}

		#[pallet::weight(T::WeightInfo::redeem_proportion())]
		#[transactional]
		pub fn redeem_proportion(
			origin: OriginFor<T>,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAssetXcm>::redeem_proportion(&who, pool_id, chain_id, amount, min_redeem_amounts)
		}

		#[pallet::weight(T::WeightInfo::redeem_single())]
		#[transactional]
		pub fn redeem_single(
			origin: OriginFor<T>,
			pool_id: StableAssetXcmPoolId,
			chain_id: ParachainId,
			amount: T::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAssetXcm>::redeem_single(&who, pool_id, chain_id, amount, i, min_redeem_amount, asset_length)
		}
	}
}

impl<T: Config> StableAssetXcm for Pallet<T> {
	type AssetId = T::AssetId;
	type Balance = T::Balance;
	type AccountId = T::AccountId;

	fn pool_count() -> StableAssetXcmPoolId {
		PoolCount::<T>::get()
	}

	fn pool(id: StableAssetPoolId) -> Option<StableAssetXcmPoolInfo<Self::AssetId, Self::Balance>> {
		Pools::<T>::get(id)
	}

	/// Create a new pool
	///
	/// # Arguments
	///
	/// * `pool_asset` - the asset ID of the pool token

	fn create_pool(pool_asset: Self::AssetId) -> DispatchResult {
		PoolCount::<T>::try_mutate(|pool_count| -> DispatchResult {
			let pool_id = *pool_count;
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				ensure!(maybe_pool_info.is_none(), Error::<T>::InconsistentStorage);
				*maybe_pool_info = Some(StableAssetXcmPoolInfo {
					pool_asset,
					balances: BTreeMap::new(),
					limits: BTreeMap::new(),
					remote_stable_assets: BTreeMap::new(),
				});
				Ok(())
			})?;

			*pool_count = pool_id.checked_add(1).ok_or(Error::<T>::InconsistentStorage)?;

			Self::deposit_event(Event::CreatePool { pool_id });
			Ok(())
		})
	}

	/// Update balance limits
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `chain_id` - the ID of remote chain
	/// * `limit` - the new balance limit

	fn update_limit(pool_id: StableAssetXcmPoolId, chain_id: ParachainId, limit: Self::Balance) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			let old_limit = pool_info.limits.get(&chain_id);
			match old_limit {
				Some(x) => ensure!(limit >= *x, Error::<T>::NewLimitInvalid),
				None => (),
			}
			pool_info.limits.insert(chain_id, limit);
			Self::deposit_event(Event::LimitUpdated {
				pool_id,
				chain_id,
				limit,
			});
			Ok(())
		})
	}

	/// Update Remote Stable Asset ID
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `chain_id` - the ID of remote chain
	/// * `remote_pool_id` - the ID of remote stable asset

	fn update_remote_stable_asset(
		pool_id: StableAssetXcmPoolId,
		chain_id: ParachainId,
		remote_pool_id: StableAssetPoolId,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			pool_info.remote_stable_assets.insert(chain_id, remote_pool_id);
			Self::deposit_event(Event::RemotePoolUpdated {
				pool_id,
				chain_id,
				remote_pool_id,
			});
			Ok(())
		})
	}

	/// Mint the pool token
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `chain_id` - the ID of remote chain
	/// * `amount` - the amount of tokens to be minted

	fn mint(
		who: Self::AccountId,
		pool_id: StableAssetXcmPoolId,
		chain_id: ParachainId,
		amount: Self::Balance,
	) -> DispatchResult {
		let result = Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			let limit = pool_info
				.limits
				.get(&chain_id)
				.copied()
				.ok_or(Error::<T>::MintOverLimit)?;
			let balance = pool_info.balances.get(&chain_id).copied().unwrap_or_else(Zero::zero);
			ensure!(balance + amount <= limit, Error::<T>::MintOverLimit);
			let new_balance = balance + amount;
			pool_info.balances.insert(chain_id, new_balance);
			T::Assets::mint_into(pool_info.pool_asset, &who, amount)?;
			Self::deposit_event(Event::Minted {
				minter: who.clone(),
				pool_id,
				chain_id,
				mint_amount: amount,
			});
			Ok(())
		});

		match result {
			Ok(_) => (),
			Err(_) => {
				let pool_info = Self::pool(pool_id).ok_or(Error::<T>::PoolNotFound)?;
				let remote_pool_id = pool_info
					.remote_stable_assets
					.get(&chain_id)
					.copied()
					.ok_or(Error::<T>::RemotePoolNotFound)?;
				T::XcmInterface::send_mint_failed(who, chain_id, remote_pool_id, amount)?;
			}
		}
		result
	}

	/// Redeem the token proportionally
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `chain_id` - the ID of remote chain
	/// * `amount` - the amount of token to be redeemed
	/// * `min_redeem_amounts` - the minimum amounts of redeemed token received

	fn redeem_proportion(
		who: &Self::AccountId,
		pool_id: StableAssetXcmPoolId,
		chain_id: ParachainId,
		amount: Self::Balance,
		min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			let balance = pool_info.balances.get(&chain_id).copied().unwrap_or_else(Zero::zero);
			ensure!(balance >= amount, Error::<T>::RedeemOverLimit);
			T::Assets::burn_from(pool_info.pool_asset, who, amount)?;
			let remote_pool_id = pool_info
				.remote_stable_assets
				.get(&chain_id)
				.copied()
				.ok_or(Error::<T>::RemotePoolNotFound)?;
			T::XcmInterface::send_redeem_proportion(
				who.clone(),
				chain_id,
				remote_pool_id,
				amount,
				min_redeem_amounts.clone(),
			)?;
			Self::deposit_event(Event::RedeemedProportion {
				redeemer: who.clone(),
				pool_id,
				chain_id,
				input_amount: amount,
				min_redeem_amounts,
			});
			Ok(())
		})
	}

	/// Redeem the token into a single token
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `chain_id` - the ID of remote chain
	/// * `amount` - the amount of token to be redeemed
	/// * `i` - the array index of the input token in StableAssetPoolInfo.assets
	/// * `min_redeem_amount` - the minimum amount of redeemed token received
	/// * `asset_length` - the length of array in StableAssetPoolInfo.assets

	fn redeem_single(
		who: &Self::AccountId,
		pool_id: StableAssetXcmPoolId,
		chain_id: ParachainId,
		amount: Self::Balance,
		i: PoolTokenIndex,
		min_redeem_amount: Self::Balance,
		asset_length: u32,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			let balance = pool_info.balances.get(&chain_id).copied().unwrap_or_else(Zero::zero);
			ensure!(balance >= amount, Error::<T>::RedeemOverLimit);
			T::Assets::burn_from(pool_info.pool_asset, who, amount)?;
			let remote_pool_id = pool_info
				.remote_stable_assets
				.get(&chain_id)
				.copied()
				.ok_or(Error::<T>::RemotePoolNotFound)?;
			T::XcmInterface::send_redeem_single(
				who.clone(),
				chain_id,
				remote_pool_id,
				amount,
				i,
				min_redeem_amount,
				asset_length,
			)?;
			Self::deposit_event(Event::RedeemedSingle {
				redeemer: who.clone(),
				pool_id,
				chain_id,
				input_amount: amount,
				i,
				min_redeem_amount,
			});
			Ok(())
		})
	}
}
