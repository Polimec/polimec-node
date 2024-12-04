import { setupWithServer } from '@acala-network/chopsticks';
import {
  type Blockchain,
  BuildBlockMode,
  connectParachains,
  connectVertical,
} from '@acala-network/chopsticks-core';
import { POLIMEC_WASM, polkadot_hub_storage, polkadot_storage } from '../overrides';

type SetupResult = Awaited<ReturnType<typeof setupWithServer>>;

export class ChainSetup {
  private relaychain?: Blockchain;
  private polimec?: Blockchain;
  private assetHub?: Blockchain;

  // Store setup objects for cleanup
  private polimecSetup?: SetupResult;
  private assetHubSetup?: SetupResult;
  private relaychainSetup?: SetupResult;

  async initialize(polimec_storage?: unknown) {
    const [polimecSetup, assetHubSetup, relaychainSetup] = await Promise.all([
      this.setupPolimec(polimec_storage),
      this.setupAssetHub(),
      this.setupRelaychain(),
    ]);

    console.log('✅ Local nodes instances are up');

    // Store setup objects
    this.polimecSetup = polimecSetup;
    this.assetHubSetup = assetHubSetup;
    this.relaychainSetup = relaychainSetup;

    // Store chain references
    this.polimec = polimecSetup.chain;
    this.assetHub = assetHubSetup.chain;
    this.relaychain = relaychainSetup.chain;

    await Promise.all([
      connectVertical(this.relaychain, this.polimec),
      connectVertical(this.relaychain, this.assetHub),
      connectParachains([this.polimec, this.assetHub]),
    ]);

    console.log('✅ HRMP channels created');
  }

  async cleanup() {
    await Promise.all([this.relaychain?.close(), this.polimec?.close(), this.assetHub?.close()]);
    await Promise.all([
      this.relaychainSetup?.close(),
      this.polimecSetup?.close(),
      this.assetHubSetup?.close(),
    ]);
    console.log('✅ Local nodes instances are down');
  }

  private async setupPolimec(polimec_storage: unknown) {
    const file = Bun.file(POLIMEC_WASM);

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

    // Initialize the Polimec setup with the provided server configuration
    return setupWithServer({
      endpoint: 'wss://polimec.ibp.network',
      port: 8000,
      'wasm-override': POLIMEC_WASM,
      'build-block-mode': BuildBlockMode.Instant,
      'import-storage': polimec_storage,
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
}
