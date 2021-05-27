#![cfg_attr(not(feature = "std"), no_std)]

extern crate sp_runtime;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use crate::traits::StableAsset;
use frame_support::codec::{Decode, Encode};
use frame_support::dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo};
use frame_support::ensure;
use frame_support::traits::Get;
use sp_runtime::traits::{
    AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
};
use frame_support::traits::fungibles::{Inspect, Mutate, Transfer,};
use sp_std::prelude::*;
use sp_std::convert::{TryFrom, };

pub type PoolTokenIndex = u32;

pub type PoolId = u32;

const NUMBER_OF_ITERATIONS_TO_CONVERGE: i32 = 255; // the number of iterations to sum d and y

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
pub struct PoolInfo<AssetId, AtLeast64BitUnsigned, Balance, AccountId> {
    pool_asset: AssetId,
    assets: Vec<AssetId>,
    precisions: Vec<AtLeast64BitUnsigned>,
    mint_fee: AtLeast64BitUnsigned,
    swap_fee: AtLeast64BitUnsigned,
    redeem_fee: AtLeast64BitUnsigned,
    total_supply: Balance,
    a: AtLeast64BitUnsigned,
    balances: Vec<Balance>,
    fee_recipient: AccountId,
    account_id: AccountId,
    pallet_id: AccountId,
}

pub mod traits {
    use crate::{PoolId, PoolInfo, PoolTokenIndex};
    use frame_support::dispatch::{DispatchResultWithPostInfo};
    use sp_std::prelude::*;

    pub trait StableAsset {
        type AssetId;
        type AtLeast64BitUnsigned;
        type Balance;
        type AccountId;

        fn pool_count() -> PoolId;

        fn pool(id: PoolId) -> Option<PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId>>;

        fn create_pool(
            who: &Self::AccountId,
            pool_asset: Self::AssetId,
            assets: Vec<Self::AssetId>,
            precisions: Vec<Self::AtLeast64BitUnsigned>,
            mint_fee: Self::AtLeast64BitUnsigned,
            swap_fee: Self::AtLeast64BitUnsigned,
            redeem_fee: Self::AtLeast64BitUnsigned,
            intial_a: Self::AtLeast64BitUnsigned,
            fee_recipient: Self::AccountId,
        ) -> DispatchResultWithPostInfo;

        fn mint(
            who: &Self::AccountId,
            pool_id: PoolId,
            amounts: Vec<Self::Balance>,
            min_mint_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        fn swap(
            who: &Self::AccountId,
            pool_id: PoolId,
            i: PoolTokenIndex,
            j: PoolTokenIndex,
            dx: Self::Balance,
            min_dy: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        fn redeem_proportion(
            who: &Self::AccountId,
            pool_id: PoolId,
            amount: Self::Balance,
            min_redeem_amounts: Vec<Self::Balance>,
        ) -> DispatchResultWithPostInfo;

        fn redeem_single(
            who: &Self::AccountId,
            pool_id: PoolId,
            amount: Self::Balance,
            i: PoolTokenIndex,
            min_redeem_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        fn redeem_multi(
            who: &Self::AccountId,
            pool_id: PoolId,
            amounts: Vec<Self::Balance>,
            max_redeem_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        fn collect_fee(
            who: &Self::AccountId,
            pool_id: PoolId,
        ) -> DispatchResultWithPostInfo;
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::{PoolId, PoolInfo, PoolTokenIndex,};
    use crate::traits::StableAsset;
    use frame_support::traits::tokens::fungibles;
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
        traits::{Currency, OnUnbalanced},
        PalletId,
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{
        CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
    };
    use sp_std::prelude::*;
    use sp_std::convert::{From, TryFrom,};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type AssetId: Parameter + Ord + Copy;
        type Balance: Parameter + Codec + Copy + Ord + From<Self::AtLeast64BitUnsigned>;
        type Assets: fungibles::Inspect<Self::AccountId, AssetId=Self::AssetId, Balance=Self::Balance> +
            fungibles::Mutate<Self::AccountId, AssetId=Self::AssetId, Balance=Self::Balance> +
            fungibles::Transfer<Self::AccountId, AssetId=Self::AssetId, Balance=Self::Balance>;
        type Currency: Currency<Self::AccountId, Balance = Self::Balance>;
        type OnUnbalanced: OnUnbalanced<
            <Self::Currency as Currency<Self::AccountId>>::NegativeImbalance,
        >;
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        type AtLeast64BitUnsigned: Parameter + CheckedAdd + CheckedSub + CheckedMul + CheckedDiv + Copy + Eq + Ord + From<Self::Balance> + From<u8> + TryFrom<usize>;
        type Precision: Get<Self::AtLeast64BitUnsigned>;
        type FeePrecision: Get<Self::AtLeast64BitUnsigned>;
        type AccountIdConvert: Convert<(Self::AccountId, PoolId), Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn pool_count)]
    pub type PoolCount<T: Config> = StorageValue<_, PoolId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> =
        StorageMap<_, Blake2_128Concat, PoolId, PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CreatePool(
            T::AccountId, // creator account id
            PoolId, // created pool id
            T::AccountId, // swap id
            T::AccountId), // pallet id
        Minted(
            T::AccountId, // minter account id
            PoolId, // minted pool id
            T::Balance, // minted amount
            Vec<T::Balance>, // mint input asset amounts
            T::Balance, // fee amount
        ),
        TokenSwapped(
            T::AccountId, // swapper account id
            PoolId, // swapped pool id
            T::AssetId, // input token asset id
            T::AssetId, // output token asset id
            T::Balance, // input token amount
            T::Balance, // output token amount
        ),
        Redeemed(
            T::AccountId, // redeemer account id
            PoolId, // redeemed pool id
            T::Balance, // redeem amount
            Vec<T::Balance>, // amount of underlying assets redeemed
            T::Balance, // fee amount
        ),
        FeeCollected(
            T::AccountId, // collector account id
            PoolId, // fee collected pool id
            T::AccountId, // fee recipient account id
            T::Balance, // collected fee amount
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        InconsistentStorage,
        ArgumentsMismatch,
        ArgumentsError,
        PoolNotFound,
        Math,
        MintUnderMin,
        SwapUnderMin,
        RedeemUnderMin,
        RedeemOverMax,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn create_pool(
            origin: OriginFor<T>,
            pool_asset: T::AssetId,
            assets: Vec<T::AssetId>,
            precisions: Vec<T::AtLeast64BitUnsigned>,
            mint_fee: T::AtLeast64BitUnsigned,
            swap_fee: T::AtLeast64BitUnsigned,
            redeem_fee: T::AtLeast64BitUnsigned,
            intial_a: T::AtLeast64BitUnsigned,
            fee_recipient: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::create_pool(&who, pool_asset, assets, precisions, mint_fee, swap_fee, redeem_fee, intial_a, fee_recipient)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn mint(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amounts: Vec<T::Balance>,
            min_mint_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::mint(&who, pool_id, amounts, min_mint_amount)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn swap(
            origin: OriginFor<T>,
            pool_id: PoolId,
            i: PoolTokenIndex,
            j: PoolTokenIndex,
            dx: T::Balance,
            min_dy: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::swap(&who, pool_id, i, j, dx, min_dy)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn redeem_proportion(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amount: T::Balance,
            min_redeem_amounts: Vec<T::Balance>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::redeem_proportion(&who, pool_id, amount, min_redeem_amounts)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn redeem_single(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amount: T::Balance,
            i: PoolTokenIndex,
            min_redeem_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::redeem_single(&who, pool_id, amount, i, min_redeem_amount)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn redeem_multi(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amounts: Vec<T::Balance>,
            max_redeem_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::redeem_multi(&who, pool_id, amounts, max_redeem_amount)
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
        pub fn collect_fee(
            origin: OriginFor<T>,
            pool_id: PoolId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as StableAsset>::collect_fee(&who, pool_id)
        }
    }

}
impl<T: Config> Pallet<T> {
    pub(crate) fn convert_number_to_balance(number: T::AtLeast64BitUnsigned) -> T::Balance {
        number.into()
    }

    pub(crate) fn convert_pool_id_to_account_id(pallet_id: T::AccountId, pool_id: PoolId) -> T::AccountId {
        <T::AccountIdConvert as Convert<(T::AccountId, PoolId), T::AccountId>>::convert((pallet_id, pool_id))
    }

    pub(crate) fn convert_balance_to_number(balance: T::Balance) -> T::AtLeast64BitUnsigned {
        balance.into()
    }

    pub(crate) fn get_number(n: u8) -> T::AtLeast64BitUnsigned {
        n.into()
    }

    pub(crate) fn convert_usize(n: usize) -> Option<T::AtLeast64BitUnsigned> {
        T::AtLeast64BitUnsigned::try_from(n).ok()
    }

    pub(crate) fn convert_vec_number_to_balance(numbers: Vec<T::AtLeast64BitUnsigned>) -> Vec<T::Balance> {
        numbers
            .into_iter()
            .map(Self::convert_number_to_balance)
            .collect()
    }

    pub(crate) fn convert_vec_balance_to_number(balances: Vec<T::Balance>) -> Vec<T::AtLeast64BitUnsigned> {
        balances
            .into_iter()
            .map(Self::convert_balance_to_number)
            .collect()
    }

    pub(crate) fn get_d(balances: &[T::AtLeast64BitUnsigned], a: T::AtLeast64BitUnsigned) -> Option<T::AtLeast64BitUnsigned> {
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        let one: T::AtLeast64BitUnsigned = Self::get_number(1);
        let mut sum: T::AtLeast64BitUnsigned = zero;
        let mut ann: T::AtLeast64BitUnsigned = a;
        let balance_size: T::AtLeast64BitUnsigned = Self::convert_usize(balances.len())?;
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
            let t3: T::AtLeast64BitUnsigned = ann.checked_sub(&one)?.checked_mul(&d)?.checked_add(&t2)?;
            d = ann.checked_mul(&sum)?
                .checked_add(&t1)?
                .checked_mul(&d)?
                .checked_div(&t3)?;
            if d > prev_d {
                if d - prev_d <= one {
                    break;
                }
            } else {
                if prev_d - d <= one {
                    break;
                }
            }
        }
        return Some(d);
    }

    pub(crate) fn get_y(balances: &[T::AtLeast64BitUnsigned], j: PoolTokenIndex, d: T::AtLeast64BitUnsigned, a: T::AtLeast64BitUnsigned) -> Option<T::AtLeast64BitUnsigned> {
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        let one: T::AtLeast64BitUnsigned = Self::get_number(1);
        let two: T::AtLeast64BitUnsigned = Self::get_number(2);
        let mut c: T::AtLeast64BitUnsigned = d;
        let mut s: T::AtLeast64BitUnsigned = zero;
        let mut ann: T::AtLeast64BitUnsigned = a;
        let balance_size: T::AtLeast64BitUnsigned = Self::convert_usize(balances.len())?;

        for i in 0..balances.len() {
            ann = ann.checked_mul(&balance_size)?;
            let j_usize = j as usize;
            if i == j_usize {
                continue;
            }
            s = s.checked_add(&balances[i])?;
            let div_op = balances[i].checked_mul(&balance_size)?;
            c = c.checked_mul(&d)?.checked_div(&div_op)?
        }

        c = c.checked_mul(&d)?.checked_div(&ann.checked_mul(&balance_size)?)?;
        let b: T::AtLeast64BitUnsigned = s.checked_add(&d.checked_div(&ann)?)?;
        let mut prev_y: T::AtLeast64BitUnsigned;
        let mut y: T::AtLeast64BitUnsigned = d;

        for _i in 0..NUMBER_OF_ITERATIONS_TO_CONVERGE {
            prev_y = y;
            y = y.checked_mul(&y)?
                .checked_add(&c)?
                .checked_div(
                    &y.checked_mul(&two)?.checked_add(&b)?.checked_sub(&d)?)?;
            if y > prev_y {
                if y - prev_y <= one {
                    break;
                }
            } else {
                if prev_y - y <= one {
                    break;
                }
            }
        }
        return Some(y);
    }

    pub(crate) fn get_mint_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>, amounts_bal: &[T::Balance]) -> Result<(T::Balance, T::Balance, Vec<T::Balance>, T::Balance), Error<T>> {
        if pool_info.balances.len() != amounts_bal.len() {
            return Err(Error::<T>::ArgumentsMismatch);
        }
        let amounts = Self::convert_vec_balance_to_number(amounts_bal.to_vec());
        let a: T::AtLeast64BitUnsigned = pool_info.a;
        let old_d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        for i in 0..balances.len() {
            if amounts[i] == zero {
                if old_d == zero {
                    return Err(Error::<T>::ArgumentsError);
                }
                continue;
            }
            let result: T::AtLeast64BitUnsigned = balances[i].checked_add(&amounts[i].checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?).ok_or(Error::<T>::Math)?;
            balances[i] = result;
        }
        let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
        let mut mint_amount: T::AtLeast64BitUnsigned = new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;
        let mut fee_amount: T::AtLeast64BitUnsigned = zero;
        let mint_fee: T::AtLeast64BitUnsigned = pool_info.mint_fee;

        if pool_info.mint_fee > zero {
            fee_amount = mint_amount.checked_mul(&mint_fee).ok_or(Error::<T>::Math)?.checked_div(&fee_denominator).ok_or(Error::<T>::Math)?;
            mint_amount = mint_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
        }

        return Ok((Self::convert_number_to_balance(mint_amount), Self::convert_number_to_balance(fee_amount), Self::convert_vec_number_to_balance(balances), Self::convert_number_to_balance(new_d)));
    }

    pub(crate) fn get_swap_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>, i: PoolTokenIndex, j: PoolTokenIndex, dx_bal: T::Balance) -> Result<(T::Balance, T::Balance, T::Balance), Error<T>> {
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        let one: T::AtLeast64BitUnsigned = Self::get_number(1);
        let balance_size: usize = pool_info.balances.len();
        let dx: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(dx_bal);
        let i_usize = i as usize;
        let j_usize = j as usize;
        if i == j {
            return Err(Error::<T>::ArgumentsError);
        }
        if dx <= zero {
            return Err(Error::<T>::ArgumentsError);
        }
        if i_usize >= balance_size {
            return Err(Error::<T>::ArgumentsError);
        }
        if j_usize >= balance_size {
            return Err(Error::<T>::ArgumentsError);
        }

        let a: T::AtLeast64BitUnsigned = pool_info.a;
        let d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        balances[i_usize] = balances[i_usize].checked_add(&dx.checked_mul(&pool_info.precisions[i_usize]).ok_or(Error::<T>::Math)?).ok_or(Error::<T>::Math)?;
        let y: T::AtLeast64BitUnsigned = Self::get_y(&balances, j, d, a).ok_or(Error::<T>::Math)?;
        let mut dy: T::AtLeast64BitUnsigned = balances[j_usize].checked_sub(&y).ok_or(Error::<T>::Math)?
            .checked_sub(&one).ok_or(Error::<T>::Math)?
            .checked_div(&pool_info.precisions[j_usize]).ok_or(Error::<T>::Math)?;
        if pool_info.swap_fee > zero {
            let fee_amount: T::AtLeast64BitUnsigned = dy.checked_mul(&pool_info.swap_fee).ok_or(Error::<T>::Math)?
                .checked_div(&fee_denominator).ok_or(Error::<T>::Math)?;
            dy = dy.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
        }
        return Ok((Self::convert_number_to_balance(dy), Self::convert_number_to_balance(y), Self::convert_number_to_balance(balances[i_usize])));
    }

    pub(crate) fn get_redeem_proportion_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>, amount_bal: T::Balance) -> Result<(Vec<T::Balance>, Vec<T::Balance>, T::Balance, T::Balance, T::Balance), Error<T>> {
        let mut amount: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(amount_bal);
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);

        if amount <= zero {
            return Err(Error::<T>::ArgumentsError);
        }

        let d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        let mut amounts: Vec<T::AtLeast64BitUnsigned> = Vec::new();
        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

        let mut fee_amount: T::AtLeast64BitUnsigned = zero;
        if pool_info.redeem_fee > zero {
            fee_amount = amount.checked_mul(&pool_info.redeem_fee).ok_or(Error::<T>::Math)?.checked_div(&fee_denominator).ok_or(Error::<T>::Math)?;
            // Redemption fee is charged with pool token before redemption.
            amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
        }

        for i in 0..pool_info.balances.len() {
            let balance_i: T::AtLeast64BitUnsigned = balances[i];
            let diff_i: T::AtLeast64BitUnsigned = balance_i.checked_mul(&amount).ok_or(Error::<T>::Math)?
                .checked_div(&d).ok_or(Error::<T>::Math)?;
            balances[i] = balance_i.checked_sub(&diff_i).ok_or(Error::<T>::Math)?;
            let amounts_i: T::AtLeast64BitUnsigned = diff_i
                .checked_div(&pool_info.precisions[i])
                .ok_or(Error::<T>::Math)?;
            amounts.push(amounts_i);
        }
        let total_supply: T::AtLeast64BitUnsigned = d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
        return Ok((Self::convert_vec_number_to_balance(amounts), Self::convert_vec_number_to_balance(balances), Self::convert_number_to_balance(fee_amount), Self::convert_number_to_balance(total_supply), Self::convert_number_to_balance(amount)));
    }

    pub(crate) fn get_redeem_single_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>, amount_bal: T::Balance, i: PoolTokenIndex) -> Result<(T::Balance, T::Balance, T::Balance, Vec<T::Balance>, T::Balance), Error<T>> {
        let mut amount: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(amount_bal);
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        let one: T::AtLeast64BitUnsigned = Self::get_number(1);
        let i_usize = i as usize;
        if amount <= zero {
            return Err(Error::<T>::ArgumentsError);
        }
        if i_usize >= pool_info.balances.len() {
            return Err(Error::<T>::ArgumentsError);
        }
        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        let a: T::AtLeast64BitUnsigned = pool_info.a;
        let d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
        let mut fee_amount: T::AtLeast64BitUnsigned = zero;

        if pool_info.redeem_fee > zero {
            fee_amount = amount.checked_mul(&pool_info.redeem_fee).ok_or(Error::<T>::Math)?
                .checked_div(&fee_denominator).ok_or(Error::<T>::Math)?;
            // Redemption fee is charged with pool token before redemption.
            amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
        }

        // The pool token amount becomes D - _amount
        let y: T::AtLeast64BitUnsigned = Self::get_y(&balances, i, d.checked_sub(&amount).ok_or(Error::<T>::Math)?, a).ok_or(Error::<T>::Math)?;
        // dy = (balance[i] - y - 1) / precisions[i] in case there was rounding errors
        let balance_i: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.balances[i_usize]);
        let dy: T::AtLeast64BitUnsigned = balance_i.checked_sub(&y).ok_or(Error::<T>::Math)?
            .checked_sub(&one).ok_or(Error::<T>::Math)?
            .checked_div(&pool_info.precisions[i_usize]).ok_or(Error::<T>::Math)?;
        let total_supply: T::AtLeast64BitUnsigned = d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
        balances[i_usize] = y;
        return Ok((Self::convert_number_to_balance(dy), Self::convert_number_to_balance(fee_amount), Self::convert_number_to_balance(total_supply), Self::convert_vec_number_to_balance(balances), Self::convert_number_to_balance(amount)));
    }

    pub(crate) fn get_redeem_multi_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>, amounts: &[T::Balance]) -> Result<(T::Balance, T::Balance, Vec<T::Balance>, T::Balance, T::Balance), Error<T>> {
        if amounts.len() != pool_info.balances.len() {
            return Err(Error::<T>::ArgumentsError);
        }
        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        let a: T::AtLeast64BitUnsigned = pool_info.a;
        let old_d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
        for i in 0..balances.len() {
            let amounts_i: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(amounts[i]);
            if amounts_i == zero {
                continue;
            }
            let balance_i: T::AtLeast64BitUnsigned = balances[i];
            // balance = balance + amount * precision
            let sub_amount: T::AtLeast64BitUnsigned = amounts_i.checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
            balances[i] = balance_i.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
        }
        let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
        let mut redeem_amount: T::AtLeast64BitUnsigned = old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
        let mut fee_amount: T::AtLeast64BitUnsigned = zero;
        if pool_info.redeem_fee > zero {
            let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
            let div_amount: T::AtLeast64BitUnsigned = fee_denominator.checked_sub(&pool_info.redeem_fee).ok_or(Error::<T>::Math)?;
            redeem_amount = redeem_amount.checked_mul(&fee_denominator).ok_or(Error::<T>::Math)?
                .checked_div(&div_amount).ok_or(Error::<T>::Math)?;
            let sub_amount: T::AtLeast64BitUnsigned = old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
            fee_amount = redeem_amount.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
        }
        let burn_amount: T::AtLeast64BitUnsigned = redeem_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
        let total_supply: T::AtLeast64BitUnsigned = old_d.checked_sub(&burn_amount).ok_or(Error::<T>::Math)?;
        return Ok((Self::convert_number_to_balance(redeem_amount), Self::convert_number_to_balance(fee_amount), Self::convert_vec_number_to_balance(balances), Self::convert_number_to_balance(total_supply), Self::convert_number_to_balance(burn_amount)));
    }

    pub(crate) fn get_pending_fee_amount(pool_info: &PoolInfo<T::AssetId, T::AtLeast64BitUnsigned, T::Balance, T::AccountId>) -> Result<(T::Balance, Vec<T::Balance>, T::Balance), Error<T>> {
        let mut balances: Vec<T::AtLeast64BitUnsigned> = Self::convert_vec_balance_to_number(pool_info.balances.clone());
        let a: T::AtLeast64BitUnsigned = pool_info.a;
        let old_d: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(pool_info.total_supply);
        for i in 0..balances.len() {
            let balance_of: T::AtLeast64BitUnsigned = Self::convert_balance_to_number(T::Assets::balance(pool_info.assets[i], &pool_info.account_id));
            balances[i] = balance_of.checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
        }
        let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
        let fee_amount: T::AtLeast64BitUnsigned = new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;

        return Ok((Self::convert_number_to_balance(fee_amount), Self::convert_vec_number_to_balance(balances), Self::convert_number_to_balance(new_d)));
    }
}

impl<T: Config> StableAsset for Pallet<T> {
    type AssetId = T::AssetId;
    type AtLeast64BitUnsigned = T::AtLeast64BitUnsigned;
    type Balance = T::Balance;
    type AccountId = T::AccountId;

    fn pool_count() -> PoolId {
        PoolCount::<T>::get()
    }

    fn pool(id: PoolId) -> Option<PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId>> {
        Pools::<T>::get(id)
    }

    fn create_pool(
        who: &Self::AccountId,
        pool_asset: Self::AssetId,
        assets: Vec<Self::AssetId>,
        precisions: Vec<Self::AtLeast64BitUnsigned>,
        mint_fee: Self::AtLeast64BitUnsigned,
        swap_fee: Self::AtLeast64BitUnsigned,
        redeem_fee: Self::AtLeast64BitUnsigned,
        intial_a: Self::AtLeast64BitUnsigned,
        fee_recipient: Self::AccountId,
    ) -> DispatchResultWithPostInfo {
        ensure!(assets.len() > 1, Error::<T>::ArgumentsError);
        ensure!(assets.len() == precisions.len(), Error::<T>::ArgumentsMismatch);
        let pool_id = PoolCount::<T>::try_mutate(|pool_count| -> Result<PoolId, DispatchError> {
            let pool_id = *pool_count;

            Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
                ensure!(maybe_pool_info.is_none(), Error::<T>::InconsistentStorage);

                let balances =
                    vec![Self::convert_number_to_balance(Self::get_number(0)); assets.len()];
                let swap_id: T::AccountId = Self::convert_pool_id_to_account_id(T::PalletId::get().into_account(), pool_id);
                frame_system::Pallet::<T>::inc_providers(&swap_id);
                *maybe_pool_info = Some(PoolInfo {
                    pool_asset: pool_asset,
                    assets: assets,
                    precisions: precisions,
                    mint_fee: mint_fee,
                    swap_fee: swap_fee,
                    redeem_fee: redeem_fee,
                    total_supply: Self::convert_number_to_balance(Self::get_number(0)),
                    a: intial_a,
                    balances: balances,
                    fee_recipient: fee_recipient,
                    account_id: swap_id,
                    pallet_id: T::PalletId::get().into_account(),
                });

                Ok(())
            })?;

            *pool_count = pool_id
                .checked_add(1)
                .ok_or(Error::<T>::InconsistentStorage)?;

            Ok(pool_id)
        })?;
        let swap_id: T::AccountId = Self::convert_pool_id_to_account_id(T::PalletId::get().into_account(), pool_id);
        Self::deposit_event(Event::CreatePool(who.clone(), pool_id, swap_id, T::PalletId::get().into_account()));

        Ok(().into())
    }

    fn mint(
        who: &Self::AccountId,
        pool_id: PoolId,
        amounts: Vec<Self::Balance>,
        min_mint_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            let mint_result = Self::get_mint_amount(&pool_info, &amounts)?;
            let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
            match mint_result {
                (mint_amount, fee_amount, balances, new_d) => {
                    ensure!(mint_amount >= min_mint_amount, Error::<T>::MintUnderMin);
                    for i in 0..amounts.len() {
                        let amount_i: Self::Balance = amounts[i];
                        if Self::convert_balance_to_number(amount_i) == zero {
                            continue;
                        }
                        T::Assets::transfer(pool_info.assets[i], who, &pool_info.account_id, amount_i, true)?;
                    }

                    T::Assets::mint_into(pool_info.pool_asset, &pool_info.fee_recipient, fee_amount)?;
                    T::Assets::mint_into(pool_info.pool_asset, who, mint_amount)?;
                    Self::deposit_event(Event::Minted(who.clone(), pool_id, mint_amount, amounts, fee_amount));
                    *maybe_pool_info = Some(PoolInfo {
                        pool_asset: pool_info.pool_asset,
                        assets: pool_info.assets,
                        precisions: pool_info.precisions,
                        mint_fee: pool_info.mint_fee,
                        swap_fee: pool_info.swap_fee,
                        redeem_fee: pool_info.redeem_fee,
                        total_supply: new_d,
                        a: pool_info.a,
                        balances: balances,
                        fee_recipient: pool_info.fee_recipient,
                        account_id: pool_info.account_id,
                        pallet_id: pool_info.pallet_id
                    });
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }

    fn swap(
        who: &Self::AccountId,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Self::Balance,
        min_dy: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            let swap_result = Self::get_swap_amount(&pool_info, i, j, dx)?;
            match swap_result {
                (dy, y, balance_i) => {
                    ensure!(dy >= min_dy, Error::<T>::SwapUnderMin);
                    let mut balances = pool_info.balances.clone();
                    let i_usize = i as usize;
                    let j_usize = j as usize;
                    balances[i_usize] = balance_i;
                    balances[j_usize] = y;
                    T::Assets::transfer(pool_info.assets[i_usize], who, &pool_info.account_id, dx, true)?;
                    T::Assets::transfer(pool_info.assets[j_usize], &pool_info.account_id, who, dy, true)?;
                    let asset_i = pool_info.assets[i_usize];
                    let asset_j = pool_info.assets[j_usize];
                    Self::deposit_event(Event::TokenSwapped(who.clone(), pool_id, asset_i, asset_j, dx, dy));
                    *maybe_pool_info = Some(PoolInfo {
                        pool_asset: pool_info.pool_asset,
                        assets: pool_info.assets,
                        precisions: pool_info.precisions,
                        mint_fee: pool_info.mint_fee,
                        swap_fee: pool_info.swap_fee,
                        redeem_fee: pool_info.redeem_fee,
                        total_supply: pool_info.total_supply,
                        a: pool_info.a,
                        balances: balances,
                        fee_recipient: pool_info.fee_recipient,
                        account_id: pool_info.account_id,
                        pallet_id: pool_info.pallet_id
                    });
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }

    fn redeem_proportion(
        who: &Self::AccountId,
        pool_id: PoolId,
        amount: Self::Balance,
        min_redeem_amounts: Vec<Self::Balance>,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            ensure!(min_redeem_amounts.len() == pool_info.assets.len(), Error::<T>::ArgumentsMismatch);
            let redeem_proportion_result = Self::get_redeem_proportion_amount(&pool_info, amount)?;
            let zero: T::Balance = Self::convert_number_to_balance(Self::get_number(0));
            match redeem_proportion_result {
                (amounts, balances, fee_amount, total_supply, redeem_amount) => {
                    for i in 0..amounts.len() {
                        ensure!(amounts[i] >= min_redeem_amounts[i], Error::<T>::RedeemUnderMin);
                        T::Assets::transfer(pool_info.assets[i], &pool_info.account_id, who, amounts[i], true)?;
                    }
                    if fee_amount > zero {
                        T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, true)?;
                    }
                    T::Assets::burn_from(pool_info.pool_asset, who, redeem_amount)?;
                    Self::deposit_event(Event::Redeemed(who.clone(), pool_id, amount, amounts, fee_amount));
                    *maybe_pool_info = Some(PoolInfo {
                        pool_asset: pool_info.pool_asset,
                        assets: pool_info.assets,
                        precisions: pool_info.precisions,
                        mint_fee: pool_info.mint_fee,
                        swap_fee: pool_info.swap_fee,
                        redeem_fee: pool_info.redeem_fee,
                        total_supply: total_supply,
                        a: pool_info.a,
                        balances: balances,
                        fee_recipient: pool_info.fee_recipient,
                        account_id: pool_info.account_id,
                        pallet_id: pool_info.pallet_id
                    });
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }

    fn redeem_single(
        who: &Self::AccountId,
        pool_id: PoolId,
        amount: Self::Balance,
        i: PoolTokenIndex,
        min_redeem_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            let redeem_single_result = Self::get_redeem_single_amount(&pool_info, amount, i)?;
            let zero: T::AtLeast64BitUnsigned = Self::get_number(0);
            let i_usize = i as usize;
            let pool_size = pool_info.assets.len();
            match redeem_single_result {
                (dy, fee_amount, total_supply, balances, redeem_amount) => {
                    ensure!(dy >= min_redeem_amount, Error::<T>::RedeemUnderMin);
                    if fee_amount > Self::convert_number_to_balance(zero) {
                        T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, true)?;
                    }
                    T::Assets::transfer(pool_info.assets[i_usize], &pool_info.account_id, who, dy, true)?;
                    T::Assets::burn_from(pool_info.pool_asset, who, redeem_amount)?;
                    let mut amounts: Vec<T::Balance> = Vec::new();
                    for idx in 0..pool_size {
                        if idx == i_usize {
                            amounts.push(dy);
                        } else {
                            amounts.push(Self::convert_number_to_balance(zero));
                        }
                    }
                    Self::deposit_event(Event::Redeemed(who.clone(), pool_id, amount, amounts, fee_amount));
                    *maybe_pool_info = Some(PoolInfo {
                        pool_asset: pool_info.pool_asset,
                        assets: pool_info.assets,
                        precisions: pool_info.precisions,
                        mint_fee: pool_info.mint_fee,
                        swap_fee: pool_info.swap_fee,
                        redeem_fee: pool_info.redeem_fee,
                        total_supply: total_supply,
                        a: pool_info.a,
                        balances: balances,
                        fee_recipient: pool_info.fee_recipient,
                        account_id: pool_info.account_id,
                        pallet_id: pool_info.pallet_id
                    });
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }

    fn redeem_multi(
        who: &Self::AccountId,
        pool_id: PoolId,
        amounts: Vec<Self::Balance>,
        max_redeem_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            let redeem_multi_result = Self::get_redeem_multi_amount(&pool_info, &amounts)?;
            let zero: T::Balance = Self::convert_number_to_balance(Self::get_number(0));
            match redeem_multi_result {
                (redeem_amount, fee_amount, balances, total_supply, burn_amount) => {
                    ensure!(redeem_amount <= max_redeem_amount, Error::<T>::RedeemOverMax);
                    if fee_amount > zero {
                        T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount, true)?;
                    }
                    for idx in 0..amounts.len() {
                        if amounts[idx] > zero {
                            T::Assets::transfer(pool_info.assets[idx], &pool_info.account_id, who, amounts[idx], true)?;
                        }
                    }
                    T::Assets::burn_from(pool_info.pool_asset, who, burn_amount)?;
                    Self::deposit_event(Event::Redeemed(who.clone(), pool_id, redeem_amount, amounts, fee_amount));
                    *maybe_pool_info = Some(PoolInfo {
                        pool_asset: pool_info.pool_asset,
                        assets: pool_info.assets,
                        precisions: pool_info.precisions,
                        mint_fee: pool_info.mint_fee,
                        swap_fee: pool_info.swap_fee,
                        redeem_fee: pool_info.redeem_fee,
                        total_supply: total_supply,
                        a: pool_info.a,
                        balances: balances,
                        fee_recipient: pool_info.fee_recipient,
                        account_id: pool_info.account_id,
                        pallet_id: pool_info.pallet_id
                    });
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }

    fn collect_fee(
        who: &Self::AccountId,
        pool_id: PoolId,
    ) -> DispatchResultWithPostInfo {
        Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
            ensure!(maybe_pool_info.is_some(), Error::<T>::PoolNotFound);
            let pool_info: PoolInfo<Self::AssetId, Self::AtLeast64BitUnsigned, Self::Balance, Self::AccountId> = maybe_pool_info.clone().unwrap();
            let fee_collect_result = Self::get_pending_fee_amount(&pool_info)?;
            let zero: T::Balance = Self::convert_number_to_balance(Self::get_number(0));
            match fee_collect_result {
                (fee_amount, balances, total_supply) => {
                    if fee_amount > zero {
                        let fee_recipient = pool_info.fee_recipient.clone();
                        T::Assets::mint_into(pool_info.pool_asset, &fee_recipient, fee_amount)?;
                        Self::deposit_event(Event::FeeCollected(who.clone(), pool_id, fee_recipient, fee_amount));
                        *maybe_pool_info = Some(PoolInfo {
                            pool_asset: pool_info.pool_asset,
                            assets: pool_info.assets,
                            precisions: pool_info.precisions,
                            mint_fee: pool_info.mint_fee,
                            swap_fee: pool_info.swap_fee,
                            redeem_fee: pool_info.redeem_fee,
                            total_supply: total_supply,
                            a: pool_info.a,
                            balances: balances,
                            fee_recipient: pool_info.fee_recipient,
                            account_id: pool_info.account_id,
                            pallet_id: pool_info.pallet_id
                        });
                    }
                    Ok(())
                }
            }

        })?;
        Ok(().into())
    }
}
