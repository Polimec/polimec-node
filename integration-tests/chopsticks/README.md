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
> - [ ] Polimec SA on AH: Add WETH balance to it in the Chopstick ovveride
> - [ ] Polimec to Asset Hub: WETH transfer
> - [ ] Polimec to Ethereum: WETH transfer
