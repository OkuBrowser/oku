use n0_error::stack_error;
use tokio::sync::mpsc;

use super::actor::NetworkMessage;

#[stack_error(derive, add_meta)]
pub struct Error;

#[derive(Debug)]
pub(super) struct RouteMonitor {
    _sender: mpsc::Sender<NetworkMessage>,
}

impl RouteMonitor {
    pub(super) fn new(_sender: mpsc::Sender<NetworkMessage>) -> Result<Self, Error> {
        // Very sad monitor. Android doesn't allow us to do this

        Ok(RouteMonitor { _sender })
    }
}
