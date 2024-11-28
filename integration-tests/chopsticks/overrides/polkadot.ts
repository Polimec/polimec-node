import { Accounts } from '../src/types';

export const polkadot_storage = {
  System: {
    Account: [
      [
        [Accounts.ALICE],
        {
          providers: 1,
          data: {
            free: '20000000000000000000',
          },
        },
      ],
    ],
  },
  ParasDisputes: {
    $removePrefix: ['disputes'],
  },
} as const;
