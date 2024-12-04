import { Accounts } from '@/types';

export const INITIAL_BALANCES = {
  USDT: 52000000000n,
  USDC: 66600000000n,
  DOT: 10000000000000000n,
  PLMC: 10000000000000000n,
} as const;

export const TRANSFER_AMOUNTS = {
  TOKENS: 2000000n,
  NATIVE: 20000000000n,
} as const;

export const DERIVE_PATHS = {
  [Accounts.ALICE]: '//Alice',
  [Accounts.BOB]: '//Bob',
};
