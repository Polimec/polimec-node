# Polimec Parachain  <!-- omit in toc -->

The **Po**lkadot **Li**quidity **Mech**anism (Polimec) is an open-source blockchain system designed to help getting liquidity into Polkadot based projects that are not yet ready to issue transferable currencies on their own technology.
It is a concept like the ERC20 Smart Contract mechanism but for issuing Pre-Currencies on Polkadot or Kusama as it enables Polkaverse projects to issue transferable pre-coins before the go-live of their Main-Net.

## Table of contents <!-- omit in toc -->

- [1. How does Polimec work?](#1-how-does-polimec-work)
  - [1.1. Main functionality](#11-main-functionality)
  - [1.2. Proposals](#12-proposals)
  - [1.3. Bonding](#13-bonding)
    - [1.3.1. Voting](#131-voting)
    - [1.3.2. Payouts](#132-payouts)
- [2. Further Documentation](#2-further-documentation)

## 1. How does Polimec work?

Polimec is planned to run as a Parachain in Polkadot.
It enables users to hold, bond and transfer multiple Currencies directly on the runtime.
Since **Polimec will not have a native token**, you can do all of these actions with just the particular Pre-Currency you are acting with!

### 1.1. Main functionality
- Apply to be an Issuer: This requires a registration fee and upon approval, your Pre-Currency is minted on the runtime.
- Transfer a Pre-Currency: As stated above, you do not need another Currency for transfering.
- Migrate all Pre-Currency to your Main-Nets Currency once it is ready.
- Bond a Pre-Currency for Voting: Enables you to take part to vote on referenda like admitting new issuers.
- Bond a Pre-Currency for Payouts: This enables you to receive payouts. This uses a different lock than the bonding for voting. Thus, you can bond of all your tokens twice. 
- Vote on Referenda: For each voting-bonded Pre-Currency, you can vote.
The final weight of your vote for Pre-Currency Y is equal to your share in the overall amount bonded for Y:
```
Voting Weight for Pre-Currency Y = your bond / overall bond
```

### 1.2. Proposals

Proposals are handled very similarily to Polkadot and Kusama with the exception that there *can be more than one public proposal*. Additionally, *each proposal has its own timeline*, e.g. each proposal can immediately be voted on for the `GetSessionDuration` and the votes are automatically tallied in block 
```
block_when_proposed + GetSessionDuration.
```

### 1.3. Bonding

As of now you can bond your tokens twice, once for voting in `pallet-multi-stake` (WIP) and once for receiving payouts in `pallet-bonding-payouts`.

Eventually this *should be handled by the frontend* to not confuse the user. More precisely, the frontend should only display one option for bonding. For unbonding, some smart logic should be used which has not yet been thoroughly thought about.

One could also remove the voting bond completely, as the payouts seemed more important as of December 2020.
#### 1.3.1. Voting

- Bond a Pre-Currency for Voting: Bonding enables you to take part in referenda (e.g. approval of new Issuer-Applicants).
- Unbond a Pre-Currency from Voting: Unbond any mount less or equal than your currently bonded one.
- `UnbondDuration`: the remainder of this era + one more full era
  - This *prevents attacks* of immediately calling unbond after bonding+voting and receiving back the tokens at the end of the era with the proposal still not having terminated. This could be possible in Polimec because proposals have their own timelines and don't necessarily follow eras as in Polkadot.
- Have to manually call unlock after unbonding and waiting out the `UnbondDuration` (as in Substrate `staking-pallet`)

#### 1.3.2. Payouts

- Bond a Pre-Currency for Payouts: Bonding enables you receive payouts for this currency if it was enabled by the Issuer and the PayoutPool (most likely the the treasury) is not empty yet.
- Each pair of (user, currency) bond runs on *its own timeline*, e.g. once a currency is bonded in `block_current`, it will receive payouts in `block_current + BondingDuration` automatically (if the payout pool is not empty). This reduces the chance of multiple payouts happening in the same block. If all users had the same timeline, this could lead to multiple blocks being filled with payouts which are free transactions (no block rewards).
- Unbond a Pre-Currency from Payouts: Unbond any mount less or equal than your currently bonded one.
- `UnbondDuration`: the remainder of this era + one more full era
   - This *prevents attacks* of immediately calling unbond after bonding and receiving back the tokens at the end of the era together with the payout. In other words: There would be no downside (except for transaction fees) to unbond immediately and have all tokens freely available after each user era while still receiving the same amount of payout as users who keep their tokens bonded.
- Unlocking happens automatically when receiving payouts, no need to call it manually.

## 2. Further Documentation

TODO