use xcm::v3::prelude::{XcmResult, XcmError};
use xcm::v3::opaque::Instruction;

pub trait HrmpHandler {
    fn handle_channel_open_request(message: Instruction) -> XcmResult;
    fn handle_channel_accepted(message: Instruction) -> XcmResult;
}

impl HrmpHandler for () {
    fn handle_channel_open_request(_message: Instruction) -> XcmResult {
        Err(XcmError::NoDeal)
    }
    fn handle_channel_accepted(_message: Instruction) -> XcmResult {
        Err(XcmError::NoDeal)
    }
}
