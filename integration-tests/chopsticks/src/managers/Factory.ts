import { Chains } from '@/types';
import { BridgerHubManagaer } from './BridgeHubManager';
import { PolimecManager } from './PolimecManager';
import { PolkadotHubManager } from './PolkadotHubManager';
import { PolkadotManager } from './PolkadotManager';

const chainManagerMap = {
  [Chains.PolkadotHub]: PolkadotHubManager,
  [Chains.Polimec]: PolimecManager,
  [Chains.Polkadot]: PolkadotManager,
  [Chains.BridgeHub]: BridgerHubManagaer,
} satisfies Record<
  Chains,
  new () => PolkadotHubManager | PolimecManager | PolkadotManager | BridgerHubManagaer
>;

export function createChainManager<T extends Chains>(
  chain: T,
): InstanceType<(typeof chainManagerMap)[T]> {
  const ManagerClass = chainManagerMap[chain];
  return new ManagerClass() as InstanceType<(typeof chainManagerMap)[T]>;
}
