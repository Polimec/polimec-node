# Polimec Parachain <!-- omit in toc -->

> **Warning** Under HEAVY development

## How to run it (Parachain Mode)

### Requirements

- [Rust](https://rustup.rs/)
- [Zombienet](https://github.com/paritytech/zombienet), install guide
  [here](https://paritytech.github.io/zombienet/install.html)

### Step 1: Compile the relay chain and add it to $PATH

- Clone the [Polkadot Repository](https://github.com/paritytech/polkadot)
- Compile it using `cargo b -r -p polkadot`
- Add the binary to your $PATH, e.g.
  `cp target/release/polkadot ~/.local/bin/polkadot`

### Step 2: Compile Polimec and add it to $PATH

- Clone this repository
- Compile it using `cargo b -r -p polimec-parachain-node`
- Add the binary to your $PATH, e.g.
  `cp target/release/polimec-parachain-node ~/.local/bin/polimec`

### Step 3: Run the network using Zombienet

- Use the `zombienet` command to run the network, e.g.
  `zombienet spawn scripts/local_parachain.toml -p native`

### TODO: Provide a Docker/Kubernetes deployment option

## How to run it (Standalone Mode)

```
$ cargo build --release
```

```
$ ./target/release/polimec-standalone-node --dev
```

or

```
$ cargo run --release -- --dev
```
