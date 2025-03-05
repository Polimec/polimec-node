import { INITIAL_BALANCES } from '@/constants';
import { Accounts, Asset } from '@/types';
import { eth_location } from './polimec';

export const polkadot_hub_storage = {
  System: {
    Account: [
      [
        [Accounts.ALICE],
        {
          providers: 1,
          data: {
            free: INITIAL_BALANCES.DOT,
          },
        },
      ],
    ],
  },
  Assets: {
    Account: [
      [
        [Asset.USDT, Accounts.ALICE],
        {
          balance: INITIAL_BALANCES.USDT,
        },
      ],
      [
        [Asset.USDC, Accounts.ALICE],
        {
          balance: INITIAL_BALANCES.USDC,
        },
      ],
    ],
  },
  ForeignAssets: {
    Account: [
      [
        [eth_location, Accounts.POLIMEC],
        {
          balance: INITIAL_BALANCES.WETH,
        },
      ],
    ],
  },
} as const;
