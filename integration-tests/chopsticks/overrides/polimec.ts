import { INITIAL_BALANCES } from '@/constants';
import { Accounts, Asset, AssetLocation, AssetSourceRelation } from '@/types';

export const POLIMEC_WASM =
  '../../target/release/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm';

const usdc_location = {
  parents: 1,
  interior: {
    x3: [{ parachain: 1000 }, { palletInstance: 50 }, { generalIndex: 1337 }],
  },
};
const usdt_location = {
  parents: 1,
  interior: {
    x3: [{ parachain: 1000 }, { palletInstance: 50 }, { generalIndex: 1984 }],
  },
};
const dot_location = {
  parents: 1,
  interior: {
    here: undefined,
  },
};

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

export const polimec_storage = {
  System: {
    Account: [
      [
        [Accounts.BOB],
        {
          providers: 1,
          data: {
            free: INITIAL_BALANCES.PLMC,
          },
        },
      ],
    ],
  },
  ForeignAssets: {
    Account: [
      [
        [usdc_location, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.USDC,
        },
      ],
      [
        [usdt_location, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.USDT,
        },
      ],
      [
        [dot_location, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.DOT,
        },
      ],
      [
        [weth_location, Accounts.BOB],
        {
          balance: INITIAL_BALANCES.WETH,
        },
      ],
    ],
    Asset: [
      // [
      //   [usdc_location],
      //   {
      //     owner: Accounts.ALICE,
      //     issuer: Accounts.ALICE,
      //     admin: Accounts.ALICE,
      //     freezer: Accounts.ALICE,
      //     supply: INITIAL_BALANCES.USDC,
      //     deposit: 0n,
      //     min_balance: 70000n,
      //     is_sufficient: true,
      //     accounts: 1,
      //     sufficients: 1,
      //     approvals: 0,
      //     status: 'Live',
      //   },
      // ],
      // [
      //   [usdt_location],
      //   {
      //     owner: Accounts.ALICE,
      //     issuer: Accounts.ALICE,
      //     admin: Accounts.ALICE,
      //     freezer: Accounts.ALICE,
      //     supply: INITIAL_BALANCES.USDT,
      //     deposit: 0n,
      //     min_balance: 70000n,
      //     is_sufficient: true,
      //     accounts: 1,
      //     sufficients: 1,
      //     approvals: 0,
      //     status: 'Live',
      //   },
      // ],
      // [
      //   [dot_location],
      //   {
      //     owner: Accounts.ALICE,
      //     issuer: Accounts.ALICE,
      //     admin: Accounts.ALICE,
      //     freezer: Accounts.ALICE,
      //     supply: INITIAL_BALANCES.DOT,
      //     deposit: 0n,
      //     min_balance: 100000000n,
      //     is_sufficient: true,
      //     accounts: 1,
      //     sufficients: 1,
      //     approvals: 0,
      //     status: 'Live',
      //   },
      // ],
      [
        [weth_location],
        {
          owner: Accounts.ALICE,
          issuer: Accounts.ALICE,
          admin: Accounts.ALICE,
          freezer: Accounts.ALICE,
          supply: INITIAL_BALANCES.WETH,
          deposit: 0n,
          min_balance: 1000000n,
          is_sufficient: true,
          accounts: 1,
          sufficients: 1,
          approvals: 0,
          status: 'Live',
        },
      ],
    ],
    Metadata: [
    //   [[usdc_location], { symbol: 'USDC', name: 'USDC', decimals: 6, isFrozen: false }],
    //   [[usdt_location], { symbol: 'USDT', name: 'USDC', decimals: 6, isFrozen: false }],
    //   [[dot_location], { symbol: 'DOT', name: 'DOT', decimals: 10, isFrozen: false }],
      [[weth_location], { symbol: 'WETH', name: 'WETH', decimals: 18, isFrozen: false }],
    ],
  },
} as const;
