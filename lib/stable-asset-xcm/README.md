# Project description

### Project Details

The Stable Asset is an asset synthetic protocol based on Curve's StableSwap algorithm as shown below:

![Stable Swap Algorithm](https://lh6.googleusercontent.com/i6owf1R5cUcc8lQtPTouisnVsj1Dt3xeCyeC_XcSjLPBCk1glLh_ZHx5GUa_f5WhsrkXJZx-PKfy8dxxrl1YjDsy-suFyXeU0vx1i6zp82lK7__NCR-HcE5cxEZ0FmaACfH8Ah7z)

Widely adopted as swap algorithm among assets with the same peg, it also works perfectly as an asset synthesis algorithm with a basket of assets with the same peg due to the following properties:

* When the prices of all underlying assets in the basket are equal, the number of each underlying assets in the baskets are equal as well. At this moment, the value of the synthetic asset equals the total number of underlying assets in the basket, and the collateral ratio reaches 100%;
* Whenever the price of any underlying asset differs from each other, the value of StableAsset is smaller than the total number of underlying assets so that the collateral ratio is larger than 100%. Since all assets in the baskets are of the same value peg, their prices should fluctuate about the peg prices with low variation expected so that the overall collateral ratio should be slightly over 100%;
* Users of the underlying swap can help to maintain the basket balance subject to underlying assets price shift.

The Stable Asset system consists of three major components: Stable Asset, Stable Swap and Stable Savings.

#### Stable Asset

Stable Asset is a synthetic asset with value peg such as BTC or USD. It's backed by a basket of assets with similar peg, and it provides more reliability and better peg compared to individual asset in the basket.

The value of Stable Asset is derived from Curve's StableSwap algorithm. When there is shift in price from individual asset in the basket, the value of Stable Asset remains unchanged: The total value of Stable Asset is always the total amount of assets in the basket when their prices are all equal.

#### Stable Swap

Stable Swap is a DEX built on top of the basket which is backing Swap Asset. It serves several purposes in the systems.

* First, it enhances the capital efficiency of the baskets. Instead of staying still, the asset basket is used as DEX;
* Second, it helps maintain peg of Stable Asset. Since the prices of individual asset might shift over time, DEX users can adjust the basket composition in order to reflect the current underlying asset value;
* Third, the trading fee, along with the Stable Asset redemption fee, provide native yield to the Stable Asset holders.

Stable Swap component is built with Curve's StableSwap algorithm with enhancement to better support the value of Stable Asset. It's different from the Curve DEX in that:

* Its value composition is calculated based on the instrinic value of the Stable Assets instead of value of the underlying assets;
* It has more robust and flexible basket management functionalities which are not required in DEX;

### Error Types
```
pub enum Error<T> {
      InconsistentStorage, -- when unexpected storage error happens
      ArgumentsMismatch, -- when assets don't match the underlying pool
      ArgumentsError, -- call function argument fails validation
      PoolNotFound, -- pool not yet exist
      Math, -- arithmetic errors
      MintUnderMin, -- mint fails when it is below provided min
      SwapUnderMin, -- swap fails when it is below provided min
      RedeemUnderMin, -- redeem fails when it is below provided min
      RedeemOverMax, -- redeem fails when it is above provided max
  }
```

### Events
```
pub enum Event<T: Config> {
      CreatePool{
			pool_id: StableAssetPoolId, 
			swap_id: T::AccountId,
			pallet_id: T::AccountId,
		},
		Minted{
			who: T::AccountId,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			input_asset: Vec<T::Balance>,
			fee: T::Balance,
		},
		TokenSwapped{
			swapper: T::AccountId,
			pool_id: StableAssetPoolId,
			input_asset: T::AssetId,
			output_asset: T::AssetId,
			input_amount: T::Balance,
			output_amount: T::Balance,
		},
		Redeemed{
			redeemer: T::AccountId,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			input_amount: Vec<T::Balance>, 
			fee: T::Balance,
		},
		FeeCollected{
			pool_id: StableAssetPoolId,
			who: T::AccountId,
			amount: T::Balance,
		},
		AModified{
			pool_id: StableAssetPoolId, 
			value: T::AtLeast64BitUnsigned,
			time: T::BlockNumber,
		},
  }
```

## License

NUTS Stable Asset is Apache 2.0 licensed.
