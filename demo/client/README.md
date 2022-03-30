# Stable Asset Example Client
This program connects to the node,
sets up asset pool and performs adding,
exchanging and removing liquidity.

## Prepare and Run

Node.js should be already installed.

Example client uses yarn as a package manager. To install yarn type:

```shell
npm install --global yarn
```

Change directory to the `client` and install dependencies:

```shell
yarn install
```

Example client assumes that you run clean local node.
To clean state of the local dev node please type:

```shell
yes | cargo run -p node -- purge-chain --dev
```

Run node as usual:

```shell
cargo run -p node -- --dev
```

In the second terminal run client (from the `client` directory):

```shell
node index.js
```

## Client API

Here we describe all extrinsics that `nutsfinance-stable-asset` pallet provides.
Example client calls every extrinsic. See [index.js](index.js) for the full code.

### Create Pool

Before creating a pool, you must create assets.
Use your underlying asset system extrinsics to do so.

Provide `createPool` extrinsic with pool asset, an array of asset IDs, precisions (defined as `10 ** (18 - token decimals)`), mint fee, swap fee, redeem fee, initial A, and fee recipient:

```javascript
api.tx.stableAsset.createPool(poolAsset, assets, precisions, mintFee, swapFee, redeemFee, intialA, feeRecipient, yieldRecipient, precision)
```

### Mint

Provide `mint` extrinsic with poolID, an array of balance of underlying assets, and the minimum amount to mint:

```javascript
api.tx.stableAsset.mint(poolId, assetAmounts, minMintAmount)
```

### Swap

Provide `swap` extrinsic with poolID, the index of input token, the index of output token, swap amount, and minimum output token amount:

```javascript
api.tx.stableAsset.swap(poolId, i, j, amount, minAmount)
```

### Redeem Proportion

Provide `redeemProportion` extrinsic with poolID, the redeeem amount, and an array of minimum amounts received for each underlying asset:

```javascript
api.tx.stableAsset.redeemSingle(poolId, amount, minAmounts)
```

### Redeem Single

Provide `redeemSingle` extrinsic with poolID, the redeeem amount, redeem token index, and the minimum amount received the underlying asset:

```javascript
api.tx.stableAsset.redeemSingle(poolId, amount, idx, minAmount)
```

### Redeem Multi

Provide `redeemMulti` extrinsic with poolID, an array of amount of underlying asset, and the max amount used for the pool asset:

```javascript
api.tx.stableAsset.redeemMulti(poolId, amounts, maxAmount)
```

### Collect Fees
This method is to collect fees for `swap` and rebalance the underlying assets.
Provide `collectFee` extrinsic with poolID:

```javascript
api.tx.stableAsset.collectFee(poolId)
```
