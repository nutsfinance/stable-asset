use crate as stable_asset;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    parameter_types,
    traits::{Currency, OnUnbalanced},
    PalletId
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::traits::Convert;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::TryFrom;
use frame_support::traits::fungibles::{Inspect, Mutate, Transfer,};
use frame_support::traits::tokens::{DepositConsequence, WithdrawConsequence,};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        StableAsset: stable_asset::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

pub type AccountId = u64;

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const StableAssetPalletId: PalletId = PalletId(*b"nuts/sta");
    pub Precision: u128 = 1000000000000000000u128;
    pub FeePrecision: u128 = 10000000000u128;
}

pub type Balance = u128;
type AtLeast64BitUnsigned = u128;

pub type AssetId = i64;

pub struct AccountIdConvert;

impl Convert<(u64, u32), u64> for AccountIdConvert {
    fn convert(a: (u64, u32)) -> u64 {
        match a {
            (pallet_id, pool_id) => pallet_id + pool_id as u64
        }
    }
}

use std::cell::RefCell;
use std::collections::HashMap;

pub struct Asset {
    total: Balance,
    balances: HashMap<AccountId, Balance>,
}

thread_local! {
    static ASSETS: RefCell<Vec<Asset>> = RefCell::new(Vec::new());
}

pub trait CreateAssets<AssetId> {
    fn create_asset() -> Result<AssetId, DispatchError>;
}

pub struct TestAssets;
impl CreateAssets<AssetId> for TestAssets {
    fn create_asset() -> Result<AssetId, DispatchError> {
        ASSETS.with(|d| -> Result<AssetId, DispatchError> {
            let mut d = d.borrow_mut();
            let id =
                AssetId::try_from(d.len()).map_err(|_| DispatchError::Other(&"Too large id"))?;
            d.push(Asset {
                total: 0,
                balances: HashMap::new(),
            });

            Ok(id)
        })
    }
}

impl Mutate<AccountId> for TestAssets {
    fn mint_into(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult {
        ASSETS.with(|d| -> DispatchResult {
            let i =
                usize::try_from(asset).map_err(|_| DispatchError::Other(&"Index out of range"))?;
            let mut d = d.borrow_mut();
            let a = d
                .get_mut(i)
                .ok_or(DispatchError::Other(&"Index out of range"))?;

            if let Some(x) = a.balances.get_mut(dest) {
                *x = x
                    .checked_add(amount)
                    .ok_or(DispatchError::Other(&"Overflow"))?;
            } else {
                a.balances.insert(*dest, amount);
            }

            a.total = a
                .total
                .checked_add(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            Ok(())
        })
    }

    fn burn_from(asset: AssetId, dest: &AccountId, amount: Balance) -> Result<Balance, DispatchError> {
        ASSETS.with(|d| -> DispatchResult {
            let i =
                usize::try_from(asset).map_err(|_| DispatchError::Other(&"Index out of range"))?;
            let mut d = d.borrow_mut();
            let a = d
                .get_mut(i)
                .ok_or(DispatchError::Other(&"Index out of range"))?;

            let x = a
                .balances
                .get_mut(dest)
                .ok_or(DispatchError::Other(&"Not found"))?;

            *x = x
                .checked_sub(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            a.total = a
                .total
                .checked_sub(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            Ok(())
        })?;
        Ok(amount)
    }
}

impl Inspect<AccountId> for TestAssets {
    type AssetId = AssetId;
    type Balance = Balance;
    fn balance(asset: AssetId, who: &AccountId) -> Balance {
        ASSETS
            .with(|d| -> Option<Balance> {
                let i = usize::try_from(asset).ok()?;
                let d = d.borrow();
                let a = d.get(i)?;
                a.balances.get(who).map(|x| *x)
            })
            .unwrap_or(0)
    }

    fn total_issuance(_asset: AssetId) -> Balance {
		todo!()
	}

	fn minimum_balance(_asset: AssetId) -> Balance {
		todo!()
    }

	fn reducible_balance(
		_asset: AssetId,
		_who: &AccountId,
		_keep_alive: bool,
	) -> Balance {
		todo!()
	}

	fn can_deposit(
		_asset: Self::AssetId,
		_who: &AccountId,
		_amount: Balance,
	) -> DepositConsequence {
		todo!()
	}

	fn can_withdraw(
		_asset: AssetId,
		_who: &AccountId,
		_amount: Balance,
	) -> WithdrawConsequence<Balance> {
		todo!()
	}
}

impl Transfer<AccountId> for TestAssets {
    fn transfer(
        asset: AssetId,
        source: &AccountId,
        dest: &AccountId,
        amount: Balance,
        _keep_alive: bool,
    ) -> Result<Balance, DispatchError> {
        Self::burn_from(asset, source, amount)?;
        Self::mint_into(asset, dest, amount)?;
        Ok(amount)
    }
}

pub struct EmptyUnbalanceHandler;

type Imbalance = <pallet_balances::Pallet<Test> as Currency<AccountId>>::NegativeImbalance;

impl OnUnbalanced<Imbalance> for EmptyUnbalanceHandler {}

impl stable_asset::Config for Test {
    type Event = Event;
    type AssetId = i64;
    type Balance = Balance;
    type Currency = Balances;
    type Assets = TestAssets;
    type OnUnbalanced = EmptyUnbalanceHandler;
    type PalletId = StableAssetPalletId;

    type AtLeast64BitUnsigned = AtLeast64BitUnsigned;
    type Precision = Precision;
    type FeePrecision = FeePrecision;
    type AccountIdConvert = AccountIdConvert;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
