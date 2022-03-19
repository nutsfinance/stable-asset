const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const fs = require('fs');
const BigNumber = require('bignumber.js');

async function main() {
    const provider = new WsProvider('ws://localhost:9944');
    const types = JSON.parse(fs.readFileSync('../custom-types.json', 'utf8'));
    const api = await ApiPromise.create({ provider, types });

    const keyring = new Keyring({type: 'sr25519'});

    const alice = keyring.addFromUri('//Alice');

    const assetA = 0;
    const assetB = 1;
    const assetC = 2;
    const assetIds = [assetA, assetB, assetC];
    const pooledAssets = [assetA, assetB];
    for (let asset of assetIds) {
        console.info(`Creating asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.create(asset, alice.address, "1"));
        console.info(`Asset ${asset} created.`);
    }

    for (let asset of pooledAssets) {
        console.info(`Minting asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.mint(asset, alice.address, "100000000"));
        console.info(`Asset ${asset} minted.`);
    }

    console.info('Creating pool...');
    await includedInBlock(alice, api.tx.stableAsset.createPool(assetC, pooledAssets,
        [1, 1],
        10000000,
        20000000,
        50000000,
        10000,
        alice.address,
        alice.address,
        10000000000));

    let poolId = 0;
    // Detect asset id of lp asset of the created pool
    let poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Account Id: ${poolInfo.accountId.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);

    console.info('Setting minter/burner');
    await includedInBlock(alice, api.tx.assets.setTeam(assetC, poolInfo.accountId, poolInfo.accountId, alice.address));

    console.info('Minting');
    await includedInBlock(alice, api.tx.stableAsset.mint(poolId, [10000000, 20000000], 0));
    poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);

    console.info('Swapping');
    await includedInBlock(alice, api.tx.stableAsset.swap(poolId, 0, 1, 5000000, 0, 2));
    poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);

    console.info('Redeeming proportion');
    await includedInBlock(alice, api.tx.stableAsset.redeemProportion(poolId, "100000", [0, 0]));
    poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);

    console.info('Redeeming single');
    await includedInBlock(alice, api.tx.stableAsset.redeemSingle(poolId, "100000", 0, 0, 2));
    poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);

    console.info('Redeeming multi');
    await includedInBlock(alice, api.tx.stableAsset.redeemMulti(poolId, [50000, 50000], "1100000000000000000"));
    poolInfo = (await api.query.stableAsset.pools(poolId)).unwrap();
    console.info(`Total Supply: ${poolInfo.totalSupply.toHuman()}`);
    console.info(`Balances: ${poolInfo.balances.toHuman()}`);
}

function includedInBlock(signer, txCall) {
    return new Promise((resolve, reject) => {
        let unsub = null;
        txCall.signAndSend(signer, (result) => {
            if (result.status.isInBlock) {
                if (unsub == null) {
                    reject('Unsub still not available');
                } else {
                    unsub();
                    resolve(result.events);
                }
            }
        }).then(x => {unsub = x;}, err => reject(err));
    });
}

(async () => {
    main().catch(e => {
        console.error(`Something went horribly wrong: ${e.message}`);
    });
})();
