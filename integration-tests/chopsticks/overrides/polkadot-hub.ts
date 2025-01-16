import { INITIAL_BALANCES } from '@/constants';
import { Accounts, Assets } from '@/types';

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
        [Assets.USDT, Accounts.ALICE],
        {
          balance: INITIAL_BALANCES.USDT,
        },
      ],
      [
        [Assets.USDC, Accounts.ALICE],
        {
          balance: INITIAL_BALANCES.USDC,
        },
      ],
      [
        [Assets.UNKNOWN, Accounts.ALICE],
        {
          balance: INITIAL_BALANCES.USDT,
        },
      ],
    ],
  },
  // TODO: Add the foreignAssets storage to give to ALICE WETH = INITIAL_BALANCES.WETH
} as const;
