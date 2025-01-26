import { INITIAL_BALANCES, WETH_ADDRESS } from '@/constants';
import { Accounts } from '@/types';

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

export const weth_location = {
  parents: 2,
  interior: {
    x2: [
      {
        globalConsensus: {
          ethereum: {
            chainId: 1n,
          },
        },
      },
      {
        accountKey20: {
          key: WETH_ADDRESS,
        },
      },
    ],
  },
};

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
    ],
    Asset: [
      [
        [weth_location],
        {
          owner: Accounts.ALICE,
          issuer: Accounts.ALICE,
          admin: Accounts.ALICE,
          freezer: Accounts.ALICE,
          supply: 100n * INITIAL_BALANCES.WETH,
          deposit: 0n,
          min_balance: 15000000000000n,
          is_sufficient: true,
          accounts: 1,
          sufficients: 1,
          approvals: 0,
          status: 'Live',
        },
      ],
    ],
    Metadata: [
      [[weth_location], { symbol: 'Wrapped Ether', name: 'WETH', decimals: 18, isFrozen: false }],
    ],
  },
} as const;
