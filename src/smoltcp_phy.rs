use crate::{rx::RxPacket, tx::TxError, Eth};
use core::intrinsics::transmute;
use smoltcp::phy::{Device, DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::Error;

/// Use this Ethernet driver with [smoltcp](https://github.com/m-labs/smoltcp)
impl<'a, 'rx, 'tx> Device<'a> for Eth<'rx, 'tx> {
    type RxToken = EthRxToken<'a>;
    type TxToken = EthTxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = crate::MTU;
        caps
    }

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let eth = unsafe { transmute::<&mut Eth<'rx, 'tx>, &mut Eth<'a, 'a>>(self) };

        let tx = EthTxToken { eth };
        let rx = match eth.recv_next() {
            Ok(packet) => EthRxToken { packet },
            Err(_) => return None,
        };
        Some((rx, tx))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let eth = unsafe { transmute::<&mut Eth<'rx, 'tx>, &mut Eth<'a, 'a>>(self) };
        Some(EthTxToken { eth })
    }
}

pub struct EthRxToken<'a> {
    packet: RxPacket<'a>,
}

impl<'a> RxToken for EthRxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<R, Error>,
    {
        let result = f(&mut self.packet);
        self.packet.free();
        result
    }
}

/// Just a reference to [`Eth`](../struct.Eth.html) for sending a
/// packet later with [`consume()`](#method.consume)
pub struct EthTxToken<'a> {
    eth: *mut Eth<'a, 'a>,
}

impl<'a> TxToken for EthTxToken<'a> {
    /// Allocate a [`Buffer`](../struct.Buffer.html), yield with
    /// `f(buffer)`, and send it as an Ethernet packet.
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<R, Error>,
    {
        let eth = unsafe { &mut *self.eth };
        match eth.send(len, f) {
            Err(TxError::WouldBlock) => Err(Error::Exhausted),
            Ok(r) => r,
        }
    }
}
