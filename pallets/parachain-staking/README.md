# DPoS Pallet for Parachain Staking

This repository contains a modified version of Moonbeam's 'parachain_staking' Substrate Pallet. The original version can be found [here](https://github.com/PureStake/moonbeam/tree/master/pallets/parachain-staking).

## Modifications
The modifications to the original pallet include the following:
1. Removed Nimbus Dependencies: The original dependencies on Nimbus have been removed. This simplifies the usage of the pallet and makes it independent of Nimbus.
2. Implemented Traits from **pallet_authorship** and **pallet_session**: To replace some functionality previously provided by Nimbus, several traits from _pallet_authorship_ and _pallet_session_ have been implemented:
    - **EventHandler** from *pallet_authorship*: This trait is used to note the block author and award them points for producing a block. The points are then used for staking purposes.
    - **SessionManager** from *pallet_session*: This trait is used to manage the start and end of sessions, as well as assemble new collators for new sessions.
    - **ShouldEndSession** from *pallet_session*: This trait is used to decide when a session should end.
    - **EstimateNextSessionRotation** from *pallet_session*: This trait is used to estimate the average session length and the current session progress, as well as estimate the next session rotation.
