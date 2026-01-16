use std::{
    net::Ipv4Addr,
    str::FromStr as _,
    sync::Arc,
    thread::{self, JoinHandle},
};

use libc::{POLLERR, POLLHUP, POLLRDHUP, pollfd};

use crate::{error::FlashResult, uds::UdsClient};

pub struct Monitor {
    uds_client: Arc<UdsClient>,
}

impl Monitor {
    pub(crate) fn new(uds_client: Arc<UdsClient>) -> Self {
        Self { uds_client }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_mut_client(&self) -> &mut UdsClient {
        unsafe { &mut *Arc::as_ptr(&self.uds_client).cast_mut() }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn get_nf_ip_addr(&mut self) -> FlashResult<Ipv4Addr> {
        Ok(Ipv4Addr::from_str(&self.get_mut_client().get_ip_addr()?)?)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn get_next_ip_addr(&mut self) -> FlashResult<Vec<Ipv4Addr>> {
        Ok(self
            .get_mut_client()
            .get_dst_ip_addr()?
            .iter()
            .map(|y| Ipv4Addr::from_str(y))
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn spawn_disconnect_handler<F>(&self, handler: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        let fd = self.uds_client.get_conn_raw_fd();
        thread::spawn(move || unsafe {
            let events = POLLHUP | POLLERR | POLLRDHUP;
            let mut pollfd = pollfd {
                fd,
                events,
                revents: 0,
            };

            loop {
                if libc::poll(&raw mut pollfd, 1, -1) > 0 && (pollfd.revents & events) != 0 {
                    handler();
                    break;
                }
            }
        })
    }
}
