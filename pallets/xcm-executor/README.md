# Why we need an xcm-executor fork?

The current xcm-executor has no way of implementing logic for the instructions:

- HrmpNewChannelOpenRequest
- HrmpChannelAccepted

This fork adds a new trait `HrmpChannelOpenRequestHandler` with a config type of the same name that can be used to implement logic for the above instructions.

All the traits and types are imported from the original executor crate. This crate only has a new xcm-executor pallet and config