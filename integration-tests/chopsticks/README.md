# XCM Integration Tests: Polkadot API x Chopsticks 

## Prerequisites

- [Bun](https://bun.sh/docs/installation)

## Usage

To install dependencies:

```bash
bun install
```

To generate the Chains descriptors:

```bash
bun papi
```

> [!NOTE]
> If you need to regenerate the Polimec descriptors (e.g. you changed something on the Runtime). You can run:
>
> ```bash
> bun papi
> bun papi add polimec --wasm ../../target/release/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm
> ```

> [!NOTE]
> If you need to regenerate the descriptors. You can delete the `.papi` folder and run:
> ```bash
> bun papi add polimec --wasm ../../target/release/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm
> bun papi add bridge -w wss://sys.ibp.network/bridgehub-polkadot
> bun papi add pah -w wss://sys.ibp.network/statemint
> bun papi add polkadot -w wss://rpc.ibp.network/polkadot
> ```

To run all the tests:

```bash
bun run test
```

To run a specific test case, e.g Polkadot to Polimec:

```bash
bun run test polkadot
```
