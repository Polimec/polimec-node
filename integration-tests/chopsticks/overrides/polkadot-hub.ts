import { INITIAL_BALANCES } from '../src/constants';
import { Accounts, Assets } from '../src/types';

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
      [
        [Accounts.BOB],
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
      [
        [Assets.USDT, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.USDT,
        },
      ],
      [
        [Assets.USDC, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.USDC,
        },
      ],
    ],
  },
} as const;
