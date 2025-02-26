# chopsticks

To install dependencies:

```bash
bun install
```

```bash
bun papi
```

> [!NOTE]
> Sometimes you need to regenerate the Polimec descriptors. To do that, run:
>
> ```bash
> bun papi add polimec --wasm ../../target/release/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm
> ```

To start the chains:

```bash
bun run dev
```

To run the tests:

```bash
bun run test
```


> [!IMPORTANT]
> TODO: Add:
> - [ ] Polimec SA on AH: Add ETH balance to it in the Chopstick ovveride
> - [ ] Polimec to Asset Hub: ETH transfer. This is a "normal" transfer_asset call.
> - [ ] Polimec to Ethereum: ETH transfer. This is a bit more complex, example extrinsic: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fhydration.ibp.network#/extrinsics/decode/0x6b0d04010100a10f040801000007464a69c7e002020907040300c02aaa39b223fe8d0a0e5c4f27ead9083c756cc200130000e8890423c78a0204010002040816040d01000001010088ca48e3e1d0f1c50bd6b504e1312d21f5bd45ed147e3c30c77eb5e4d63bdc6310010102020907040300c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000201090704081300010300c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20004000d010204000103001501c1413e4178c38567ada8945a80351f7b849600
