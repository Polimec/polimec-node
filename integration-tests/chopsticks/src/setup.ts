import { expect } from 'bun:test';
import { setupWithServer } from '@acala-network/chopsticks';
import {
  type Blockchain,
  BuildBlockMode,
  connectParachains,
  connectVertical,
} from '@acala-network/chopsticks-core';
import { POLIMEC_WASM, bridge_storage, polkadot_hub_storage, polkadot_storage } from '../overrides';

type SetupResult = Awaited<ReturnType<typeof setupWithServer>>;

export class ChainSetup {
  private relaychain?: Blockchain;
  private polimec?: Blockchain;
  private assetHub?: Blockchain;
  private bridgeHub?: Blockchain;

  // Store setup objects for cleanup
  private polimecSetup?: SetupResult;
  private assetHubSetup?: SetupResult;
  private relaychainSetup?: SetupResult;
  private bridgeHubSetup?: SetupResult;

  async initialize(polimec_storage?: unknown) {
    const [polimecSetup, assetHubSetup, relaychainSetup, bridgeHubSetup] = await Promise.all([
      this.setupPolimec(polimec_storage),
      this.setupAssetHub(),
      this.setupRelaychain(),
      this.setupBridgeHub(),
    ]);

    console.log('✅ Local nodes instances are up');

    // Store setup objects
    this.polimecSetup = polimecSetup;
    this.assetHubSetup = assetHubSetup;
    this.relaychainSetup = relaychainSetup;
    this.bridgeHubSetup = bridgeHubSetup;

    // Store chain references
    this.polimec = polimecSetup.chain;
    this.assetHub = assetHubSetup.chain;
    this.relaychain = relaychainSetup.chain;
    this.bridgeHub = bridgeHubSetup.chain;

    await Promise.all([
      connectVertical(this.relaychain, this.polimec),
      connectVertical(this.relaychain, this.assetHub),
      connectVertical(this.relaychain, this.bridgeHub),
      connectParachains([this.polimec, this.assetHub]),
      connectParachains([this.bridgeHub, this.assetHub]),
    ]);

    console.log('✅ HRMP channels created');

    // Needed to execute storage migrations within the new WASM before running tests.
    const head = this.polimec.head;
    console.log(`✅ Polimec chain is at block ${head.number}`);
    console.log('✅ Producing a new block...');
    const new_block = await this.polimec?.newBlock();
    console.log(`✅ Polimec chain is at block ${new_block.number}`);
    expect(new_block.number === head.number + 1, 'Block number should be incremented by 1');
  }

  async cleanup() {
    await Promise.all([
      this.relaychain?.close(),
      this.polimec?.close(),
      this.assetHub?.close(),
      this.bridgeHub?.close(),
    ]);
    await Promise.all([
      this.relaychainSetup?.close(),
      this.polimecSetup?.close(),
      this.assetHubSetup?.close(),
      this.bridgeHubSetup?.close(),
    ]);
    console.log('✅ Local nodes instances are down');
  }

  private async setupPolimec(polimec_storage: unknown) {
    const file = Bun.file(POLIMEC_WASM);

    // Note: the tests are intended to use a pre-production, locally compiled runtime, that's why we throw an error.
    if (!(await file.exists())) {
      throw new Error(
        'Polimec runtime not found! Please build it by running `cargo b -r -p polimec-runtime` before executing the tests.',
      );
    }

    const hasher = new Bun.CryptoHasher('blake2b256');
    hasher.update(await file.bytes());
    const runtimeHash = hasher.digest('hex');
    console.log(`✅ Polimec runtime used in tests: 0x${runtimeHash}`);

    if (polimec_storage !== undefined) console.info('✅ Polimec custom storage provided');

    return setupWithServer({
      endpoint: 'wss://polimec.ibp.network',
      port: 8000,
      'wasm-override': POLIMEC_WASM,
      'import-storage': polimec_storage,
      'build-block-mode': BuildBlockMode.Instant,
    });
  }

  private setupAssetHub() {
    return setupWithServer({
      endpoint: 'wss://sys.ibp.network/statemint',
      port: 8001,
      'import-storage': polkadot_hub_storage,
      'build-block-mode': BuildBlockMode.Instant,
    });
  }

  private setupRelaychain() {
    return setupWithServer({
      endpoint: 'wss://rpc.ibp.network/polkadot',
      port: 8002,
      'import-storage': polkadot_storage,
      'build-block-mode': BuildBlockMode.Instant,
    });
  }

  private setupBridgeHub() {
    return setupWithServer({
      endpoint: 'wss://sys.ibp.network/bridgehub-polkadot',
      port: 8003,
      'import-storage': bridge_storage,
      'build-block-mode': BuildBlockMode.Instant,
    });
  }
}
