import { INITIAL_BALANCES } from '@/constants';
import { Accounts } from '@/types';

export const bridge_storage = {
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
} as const;
