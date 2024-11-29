import { INITIAL_BALANCES } from '../src/constants';
import { Accounts } from '../src/types';

export const polkadot_storage = {
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
  ParasDisputes: {
    $removePrefix: ['disputes'],
  },
} as const;
