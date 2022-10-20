# NUTS Stable Asset Pallet
![Web3 Grant](https://user-images.githubusercontent.com/2569558/196871922-a1b84ae5-2046-48b5-8ccd-e28a6dcb1bfa.jpg)

## Overview

There are vastly emerging assets in the Polkadot ecosystem, including both Polkadot native assets and assets bridged from other blockchains such as Ethereum and EOS. These assets introduce diversity in architecture and business model, but also fragmentizes the ecosystem since applications need to build separate markets for each of these assets. For example, stables coins can be divided into three categories: fiat-backed, crypto-backed and algorithmic stable coins, and on Ethereum each category has more than ten stable coin protocols. DEX benefits from such asset diversification but other protocols such as lending and options find it difficult to accommodate all these various assets.

Asset synthesis is a common approach to unify asset values and hedge asset risks. One approach is to synthesize several mainstream assets or assets belonging to the same niche so that the synthetic assets represents the general trend of the underlying assets. In this approach the synthetic assets acts similiar to an index fund, and how to fairly price and adopt the synthetic assets becomes a new question. The second approach is to synthetize several assets of the same value peg such as BTC, ETH or USD. The synthetic asset has the same value peg, and it could simplifies financial application development since only one synthetic asset needs to be supported for each peg type.

Stable Asset is an asset synthetic protocol of the second approach. It is also built with integrated swap and saving functionalities using the basket of assets.

## Installation

Make sure you have done all steps described in [Installation page](https://substrate.dev/docs/en/knowledgebase/getting-started/) of the Substrate Developer Hub.

To build project run:

```bash
cargo build
```

## Tests

To run unit tests type:

```bash
cargo test
```


In case you want run code coverage tool, please follow [instructions](https://github.com/xd009642/tarpaulin#installation) to install tarpaulin.

To create code coverage report run:

```bash
cargo tarpaulin -v
```

## Running the Node

First of all please ensure that your development chain's state is empty:

```bash
cargo run --bin node purge-chain --dev
```

Now you can start the development chain:

```bash
cargo run --bin node --dev
```
### Use Docker
You can build the docker image using `docker build -t stable-asset .`. Then run with `docker run -p 9944:9944 stable-asset`.

## Connecting to the Node

### Polkadot.js Explorer

It can be very useful to connect UI to the node you just started.

To do this open https://polkadot.js.org/apps/#/explorer in your browser first.

Follow these steps to register required custom types:

* In the main menu choose Settings tab;
* In the Settings submenu choose Developer tab;
* Copy content of the [custom-types.json](demo/custom-types.json) file into text box on the page;
* Press Save button.

### Example Client

Example client connects to the clean dev node and performs various operations with `nutsfinance-stable-asset` pallet.
See [this readme](demo/client/README.md) for details.

## Using the Pallet

- See [Client API](demo/client/README.md#client-api) for how to use the pallet from the client perspective.

## Development Roadmap

| Milestone # | Description |
| --- | --- |
| 1 | Implement [core Stable Swap algorithm](https://docs.acoconut.fi/asset/acbtc/algorithm) to maintain balance of the basket, e.g.<br>computeD<br>computeDy<br>computeSwapAmount<br>swap.<br> Part of the algorithm is implemented in Solidity in acBTC's [ACoconutSwap](https://github.com/nutsfinance/acBTC/blob/master/contracts/acoconut/ACoconutSwap.sol) contract |
| 2 | Implement core functionalities for Stable Asset, which includes both how Stable Asset is minted/redeemed, e.g.  <br>getMintAmount<br>mint<br>getRedeemProportionAmount<br>redeemProportion<br>getRedeemSingleAmount<br>redeemSingle<br>getRedeemMultiAmount<br>redeemMulti,<br> and how the basket assets are managed. The first part is partly implemented in Solidity in acBTC's [ACoconutSwap](https://github.com/nutsfinance/acBTC/blob/master/contracts/acoconut/ACoconutSwap.sol) contract |

## License

NUTS Stable Asset is [Apache 2.0 licensed](LICENSE).
