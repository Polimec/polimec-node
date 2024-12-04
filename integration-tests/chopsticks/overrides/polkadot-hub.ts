import { INITIAL_BALANCES } from '@/constants';
import { Accounts, Asset } from '@/types';


const weth_location = {
  parents: 2,
  interior: {
    x2: [
      {
        globalConsensus: {
          ethereum: {
            chainId: 1n
          }
        }
      },
      {
        accountKey20: {
          key: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
        }
      }
    ]
  }
}

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
      [[weth_location, Accounts.ALICE], { balance: 10000000000000 }]
    ],
    Asset: [
      [[weth_location], { supply: 10000000000000 }]
    ]
  }
} as const;
