import { Accounts } from '@/types';
import { FixedSizeBinary } from 'polkadot-api';

export const INITIAL_BALANCES = {
  USDT: 52000n * 10n ** 6n,
  USDC: 66000n * 10n ** 6n,
  DOT: 1000000n * 10n ** 10n,
  PLMC: 1000000n * 10n ** 10n,
  ETH: 2n * 10n ** 18n,
} as const;

export const TRANSFER_AMOUNTS = {
  TOKENS: 2n * 10n ** 6n, // e.g. 2 USDC
  NATIVE: 2n * 10n ** 10n, // e.g. 2 DOT
  BRIDGED: 1n * 10n ** 17n, // e.g. 0.1 ETH
} as const;

export const DERIVE_PATHS = {
  [Accounts.ALICE]: '//Alice',
  [Accounts.BOB]: '//Bob',
} as const;

export const ETH_ADDRESS = '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2';
export const DEFAULT_TOPIC = FixedSizeBinary.fromArray(Array(32).fill(1));
export const FEE_AMOUNT = 40_000_000_000n;
export const ETH_AMOUNT = 15_000_000_000_000n;
