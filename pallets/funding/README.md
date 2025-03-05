<!-- cargo-rdme start -->

# Funding Pallet

Polimec's main business logic. It allows users to create, evaluate, and fund projects.

Participants get contribution tokens. Contribution tokens are tokens issued by projects which successfully raised
funds on Polimec. They are distributed to evaluators and participants who contributed to the project’s successful
funding round. Contribution tokens are transferability-locked in the wallet of participants or evaluators after
distribution and are automatically converted to the project’s transferable mainnet token at launch.

## Overview
The official logic for Polimec's blockchain can be found in our [knowledge hub](https://hub.polimec.org/).

There are 3 types of users in Polimec:
- **Issuers**: They create projects and are responsible for their success.
- **Evaluators**: They are incentivized to assess projects accurately by locking their PLMC. If at least 10% of its
    target funding (in USD) is locked in PLMC, a project is given access to the funding round. Evaluators are either
    rewarded in contribution tokens if the project gets funded, or have their PLMC slashed otherwise.
- **Bidders**: They contribute financially to projects by locking PLMC and paying out USDT/USDC/DOT, and are rewarded in contribution tokens.

Users need to go through a KYC/AML by a third party in order to use the protocol. This process classifies them
into one of the following categories, based on their investment experience and financial status:
- **Institutional**
- **Professional**
- **Retail**

Basic flow of a project's lifecycle:
1) **Project Creation**: Issuer creates a project with the [`create_project`](Pallet::create_project) extrinsic.
2) **Evaluation Start**: Issuer starts the evaluation round with the [`start_evaluation`](Pallet::start_evaluation) extrinsic.
3) **Evaluate**: Evaluators bond PLMC to evaluate a project with the [`evaluate`](Pallet::evaluate) extrinsic.
4) **Evaluation End**: Anyone can end the evaluation round with the [`end_evaluation`](Pallet::end_evaluation) extrinsic after the defined end block.
5) **Auction Start**: If the project receives at least 10% of its target funding (in USD) in PLMC bonded, the auction starts immediately after `end_evaluation` is called.
6) **Bid**: Investors can place bids on the project using the [`bid`](Pallet::bid) extrinsic. The price starts at the issuer-defined minimum, and increases by increments of 10% in price and bucket size.
7) **Funding End**: Anyone can end the project with the [`end_project`](Pallet::end_project) extrinsic after the defined end block.
    The project will now be considered Failed if it reached <=33% of its target funding in USD, and Successful otherwise.
8) **Settlement Start**: Anyone can start the settlement process with the [`start_settlement`](Pallet::start_settlement) extrinsic after the defined end block.
9) **Settle Evaluation**: Anyone can now settle an evaluation with the [`settle_evaluation`](Pallet::settle_evaluation) extrinsic.
    This will unlock the PLMC bonded, and either apply a slash to the PLMC, or reward CTs to the evaluator.
10) **Settle Bid**: Anyone can now settle a bid with the [`settle_bid`](Pallet::settle_bid) extrinsic.
    This will set a vesting schedule on the PLMC bonded, and pay out the funding assets to the issuer. It will also issue refunds in case the bid failed,
    or the price paid was higher than the weighted average price.
11) **Settlement End**: Anyone can now mark the project settlement as finished by calling the [`mark_project_as_settled`](Pallet::mark_project_as_settled) extrinsic.
12) **Migration Start**: Once the issuer has tokens to distribute on mainnet, he can start the migration process with the [`start_offchain`](Pallet::start_offchain_migration) extrinsic.
13) **Confirm Migration**: The issuer has to mark each participant's CTs as migrated with the [`confirm_offchain_migration`](Pallet::confirm_offchain_migration) extrinsic.
14) **Migration End**: Once all participants have migrated their CTs, anyone can mark the migration as finished with the [`mark_project_ct_migration_as_finished`](Pallet::mark_project_ct_migration_as_finished) extrinsic.

<!-- cargo-rdme end -->
