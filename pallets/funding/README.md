 # Funding Pallet

 Polimec's main business logic. It allows users to create, evaluate, and fund projects.

 It rewards project evaluators and contributors with `Contribution Tokens`. These tokens
 can be redeemed for a project's native tokens, after their parachain is deployed on mainnet.

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

 A project that is successfully funded, goes through the following flow:

 | Step                   | Description                                                                                                                                                                              | Resulting Project State                                             |
 |------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------|
 | Creation               | Issuer creates a project with the [`create()`](./src/lib.rs#L255) extrinsic                                                                                                              | [`Application`](ProjectStatus::Application)                         |
 | Evaluation Start       | Issuer starts the evaluation round with the [`start_evaluation()`](Pallet::start_evaluation) extrinsic.                                                                                  | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
 | Evaluation Submissions | Evaluators assess the project information, and if they think it is good enough to get funding, they bond Polimec's native token PLMC with [`bond_evaluation()`](Pallet::bond_evaluation) | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
 | Evaluation End         | Evaluation round ends automatically after the [`Config::EvaluationDuration`] has passed. This is achieved by the [`on_initialize()`](Pallet::on_initialize) function.                    | [`AuctionInitializePeriod`](ProjectStatus::AuctionInitializePeriod) |
 | Auction Start          | Issuer starts the auction round with the [`start_auction()`](Pallet::start_auction) extrinsic.                                                                                           | [`AuctionRound(Candle)`](ProjectStatus::AuctionRound)               |
 | Auction Start          | Issuer starts the auction round with the [`start_auction()`](Pallet::start_auction) extrinsic.                                                                                           | [`AuctionRound(English)`](ProjectStatus::AuctionRound)              |

 - **Auction Start**: The Project is now in the
 ## Interface

 ### Permissioned Functions, callable only by credentialized users

 * `note_image` : Save on-chin the Hash of the project metadata.
 * `create` : Create a new project.
 * `bond_evaluation` : Bond PLMC on a project's evaluation round.
 * `failed_evaluation_unbond_for` : Unbond the PLMC bonded on a project's evaluation round for any user, if the project failed the evaluation.
 * `bid` : Perform a bid during the Auction Round.
 * `contribute` : Contribute to a project during the Community Round.
 * `claim_contribution_tokens` : Claim the Contribution Tokens if you contributed to a project during the Funding Round.

 ### Privileged Functions, callable only by the project's Issuer

 * `edit_metadata` : Submit a new Hash of the project metadata.
 * `start_evaluation` : Start the Evaluation Round of a project.
 * `start_auction` : Start the Funding Round of a project.

