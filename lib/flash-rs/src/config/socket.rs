use crate::mem::PollOutStatus;

use super::{poll::PollConfig, xsk::XskConfig};

#[derive(Debug)]
pub(crate) struct SocketConfig {
    pub(crate) xsk: XskConfig,
    pub(crate) poll: PollConfig,
    pub(crate) pollout_status: PollOutStatus,
}

impl SocketConfig {
    pub(crate) fn new(xsk: XskConfig, poll: PollConfig, pollout_status: PollOutStatus) -> Self {
        Self {
            xsk,
            poll,
            pollout_status,
        }
    }
}
