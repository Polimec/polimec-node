# Integration Tests with `xcm-emulator`

This folder contains integration tests that use the xcm-emulator to verify if
specific XCM messages work in live networks. The xcm-emulator uses production
relay chain and parachain runtime and supports different runtime environments
like Kusama, Statemine, etc. The emulator is also capable of keeping up-to-date
with the latest chain specs to provide a realistic testing environment.

## Overview

![](https://i.imgur.com/8f0g8yG.jpg)

## How to test

```bash
$ cd polimec-node/integration-tests/
$ cargo test --features std,testing-node,fast-gov
```
