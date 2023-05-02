# Funding Pallet 

Polimec's main business logic. It allows users to create, evaluate, and fund projects.

It rewards project evaluators and contributors with `Contribution Tokens`. These tokens
can be redeemed for a project's native tokens, after their parachain is deployed on mainnet.
## 👷 Work in progress 🏗️
Expect major changes between PRs

## Overview
The official logic for Polimec's blockchain can be found at our [whitepaper](https://polimec.link/whitepaper).

There are 3 types of users in Polimec:
- **Issuers**: They create projects and are responsible for their success.
- **Evaluators**: They evaluate projects and are rewarded for their work.
- **Contributors**: They contribute financially to projects and are rewarded on the basis of their contribution

A contributor, depending on their investor profile, can participate in different rounds of a project's funding.

There are 3 types of contributors:
- **Institutional**
- **Professional**
- **Retail**

Basic flow of a project's lifecycle:

| Step                      | Description                                                                                                                                                                                                                                                                                                                                                                                                 | Resulting Project State                                             |
|---------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------|
| Creation                  | Issuer creates a project with the [`create()`](Pallet::create) extrinsic.                                                                                                                                                                                                                                                                                                                                   | [`Application`](ProjectStatus::Application)                         |
| Evaluation Start          | Issuer starts the evaluation round with the [`start_evaluation()`](Pallet::start_evaluation) extrinsic.                                                                                                                                                                                                                                                                                                     | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
| Evaluation Submissions    | Evaluators assess the project information, and if they think it is good enough to get funding, they bond Polimec's native token PLMC with [`bond_evaluation()`](Pallet::bond_evaluation)                                                                                                                                                                                                                    | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
| Evaluation End            | Evaluation round ends automatically after the [`Config::EvaluationDuration`] has passed. This is achieved by the [`on_initialize()`](Pallet::on_initialize) function.                                                                                                                                                                                                                                       | [`AuctionInitializePeriod`](ProjectStatus::AuctionInitializePeriod) |
| Auction Start             | Issuer starts the auction round within the [`Config::AuctionInitializePeriodDuration`], by calling the extrinsic [`start_auction()`](Pallet::start_auction)                                                                                                                                                                                                                                                 | [`AuctionRound(English)`](ProjectStatus::AuctionRound)              |
| Bid Submissions           | Institutional and Professional users can place bids with [`bid()`](Pallet::bid) by choosing their desired token price, amount, and multiplier (for vesting). Their bids are guaranteed to be considered                                                                                                                                                                                                     | [`AuctionRound(English)`](ProjectStatus::AuctionRound)              |                                                                                                                                                                                                                |                                                                     |
| Candle Auction Transition | After the [`Config::EnglishAuctionDuration`] has passed, the auction goes into candle mode thanks to [`on_initialize()`](Pallet::on_initialize)                                                                                                                                                                                                                                                             | [`AuctionRound(Candle)`](ProjectStatus::AuctionRound)               |
| Bid Submissions           | Institutional and Professional users can continue bidding, but this time their bids will only be considered, if they managed to fall before the random ending block calculated at the end of the auction.                                                                                                                                                                                                   | [`AuctionRound(Candle)`](ProjectStatus::AuctionRound)               |
| Community Funding Start   | After the [`Config::CandleAuctionDuration`] has passed, the auction automatically. A final token price for the next rounds is calculated based on the accepted bids.                                                                                                                                                                                                                                        | [`CommunityRound`](ProjectStatus::CommunityRound)                   |
| Funding Submissions       | Retail investors can call the [`contribute()`](Pallet::contribute) extrinsic to buy tokens at the set price.                                                                                                                                                                                                                                                                                                | [`CommunityRound`](ProjectStatus::CommunityRound)                   |
| Remainder Funding Start   | After the [`Config::CommunityFundingDuration`] has passed, the project is now open to token purchases from any user type                                                                                                                                                                                                                                                                                    | [`RemainderRound`](ProjectStatus::RemainderRound)                   |
| Funding End               | If all tokens were sold, or after the [`Config::RemainderFundingDuration`] has passed, the project automatically ends, and it is calculated if it reached its desired funding or not.                                                                                                                                                                                                                       | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
| Evaluator Rewards         | If the funding was successful, evaluators can claim their contribution token rewards with the [`TBD`]() extrinsic. If it failed, evaluators can either call the [`failed_evaluation_unbond_for()`](Pallet::failed_evaluation_unbond_for) extrinsic, or wait for the [`on_idle()`](Pallet::on_initialize) function, to return their funds                                                                    | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
| Bidder Rewards            | If the funding was successful, bidders will call [`vested_contribution_token_bid_mint_for()`](Pallet::vested_contribution_token_bid_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_bid_unbond_for()`](Pallet::vested_plmc_bid_unbond_for) to unbond their PLMC, based on their current vesting schedule.                                                                        | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
| Buyer Rewards             | If the funding was successful, users who bought tokens on the Community or Remainder round, can call [`vested_contribution_token_purchase_mint_for()`](Pallet::vested_contribution_token_purchase_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_purchase_unbond_for()`](Pallet::vested_plmc_purchase_unbond_for) to unbond their PLMC, based on their current vesting schedule | [`FundingEnded`](ProjectStatus::FundingEnded)                       |                                                                   |

## Interface
All users who wish to participate need to have a valid credential, given to them on the KILT parachain, by a KYC/AML provider.
### Extrinsics
* `create` : Creates a new project.
* `edit_metadata` : Submit a new Hash of the project metadata.
* `start_evaluation` : Start the Evaluation round of a project.
* `start_auction` : Start the English Auction round of a project.
* `bond_evaluation` : Bond PLMC on a project in the evaluation stage. A sort of "bet" that you think the project will be funded
* `failed_evaluation_unbond_for` : Unbond the PLMC bonded on a project's evaluation round for any user, if the project failed the evaluation.
* `bid` : Perform a bid during the English or Candle Auction Round.
* `contribute` : Buy contribution tokens if a project during the Community or Remainder round
* `vested_plmc_bid_unbond_for` : Unbond the PLMC bonded on a project's English or Candle Auction Round for any user, based on their vesting schedule.
* `vested_plmc_purchase_unbond_for` : Unbond the PLMC bonded on a project's Community or Remainder Round for any user, based on their vesting schedule.
* `vested_contribution_token_bid_mint_for` : Mint the contribution tokens for a user who participated in the English or Candle Auction Round, based on their vesting schedule.
* `vested_contribution_token_purchase_mint_for` : Mint the contribution tokens for a user who participated in the Community or Remainder Round, based on their vesting schedule.

### Storage Items
* `NextProjectId` : Increasing counter to get the next id to assign to a project.
* `NextBidId`: Increasing counter to get the next id to assign to a bid.
* `Nonce`: Increasing counter to be used in random number generation.
* `Images`: Map of the hash of some metadata to the user who owns it. Avoids storing the same image twice, and keeps track of ownership for a future project data access due to regulatory compliance.
* `Projects`: Map of the assigned id, to the main information of a project.
* `ProjectsIssuers`: Map of a project id, to its issuer account.
* `ProjectsInfo`: Map of a project id, to some additional information required for ensuring correctness of the protocol.
* `ProjectsToUpdate`: Map of a block number, to a vector of project ids. Used to keep track of projects that need to be updated in on_initialize.
* `AuctionsInfo`: Double map linking a project-user to the bids they made.
* `EvaluationBonds`: Double map linking a project-user to the PLMC they bonded in the evaluation round.
* `BiddingBonds`: Double map linking a project-user to the PLMC they bonded in the English or Candle Auction round.
* `ContributingBonds`: Double map linking a project-user to the PLMC they bonded in the Community or Remainder round.
* `Contributions`: Double map linking a project-user to the contribution tokens they bought in the Community or Remainder round.

## Usage
You can circumvent the extrinsics by calling the do_* functions that they call directly.
This is useful if you need to make use of this pallet's functionalities in a pallet of your own, and you don't want to pay the transaction fees twice.
### Example: A retail user buying tokens for a project in the community round
```rust
pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

use pallet_funding::{self as funding};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + funding::Config {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Buy tokens for a project in the community round if it achieved at least 500k USDT funding
        #[pallet::weight(0)]
        pub fn buy_if_popular(
            origin: OriginFor<T>,
            project_id: <T as funding::Config>::ProjectIdParameter,
            amount: <T as funding::Config>::CurrencyBalance
        ) -> DispatchResult {
            let retail_user = ensure_signed(origin)?;
            let project_id: <T as funding::Config>::ProjectIdentifier = project_id.into();
            // Check project is in the community round
            let project_info = funding::Pallet::<T>::project_info(project_id).ok_or(Error::<T>::ProjectNotFound)?;
            ensure!(project_info.project_status == funding::ProjectStatus::CommunityRound, "Project is not in the community round");

            // Calculate how much funding was done already
            let project_contributions: <T as funding::Config>::CurrencyBalance = funding::Contributions::<T>::iter_prefix_values(project_id)
                .flatten()
                .fold(
                    0u64.into(),
                    |total_tokens_bought, contribution| {
                        total_tokens_bought + contribution.contribution_amount
                    }
                );

            ensure!(project_contributions >= 500_000_0_000_000_000u64.into(), "Project did not achieve at least 500k USDT funding");

            // Buy tokens with the default multiplier
            <funding::Pallet<T>>::do_contribute(retail_user, project_id, amount, None)?;

            Ok(())
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        ProjectNotFound,
    }
}
```

## Credentials
The pallet will only allow users with certain credential types, to execute certain extrinsics.:


| Extrinsic                                     | Issuer | Retail Investor | Professional Investor | Institutional Investor |
|-----------------------------------------------|--------|-----------------|-----------------------|------------------------|
| `create`                                      | X      |                 |                       |                        |
| `edit_metadata`                               | X      |                 |                       |                        |
| `start_evaluation`                            | X      |                 |                       |                        |
| `start_auction`                               | X      |                 |                       |                        |
| `bond_evaluation`                             |        | X               | X                     | X                      |
| `failed_evaluation_unbond_for`                |        | X               | X                     | X                      |
| `bid`                                         |        |                 | X                     | X                      |
| `contribute`                                  |        | X               | X*                    | X*                     |
| `vested_plmc_bid_unbond_for`                  |        |                 | X                     | X                      |
| `vested_plmc_purchase_unbond_for`             |        | X               | X                     | X                      |
| `vested_contribution_token_bid_mint_for`      |        |                 | X                     | X                      |
| `vested_contribution_token_purchase_mint_for` |        | X               | X                     | X                      |

_* They can call contribute only if the project is on the remainder round._ 


