import { Accounts, Assets } from '../src/types';

export const polkadot_hub_storage = {
  System: {
    Account: [
      [
        [Accounts.ALICE],
        {
          providers: 1,
          data: {
            free: '10000000000000000',
          },
        },
      ],
      [
        [Accounts.BOB],
        {
          providers: 1,
          data: {
            free: '10000000000000000',
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
          balance: 52000000000,
        },
      ],
      [
        [Assets.USDC, Accounts.ALICE],
        {
          balance: 66600000000,
        },
      ],
      [
        [Assets.USDT, Accounts.BOB],
        {
          balance: 66600000000,
        },
      ],
      [
        [Assets.USDC, Accounts.BOB],
        {
          balance: 66600000000,
        },
      ],
    ],
  },
} as const;
