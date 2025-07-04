use crate::timer::current_time_duration;
use alloc::vec;
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet},
    phy::{Device, Loopback, Medium},
    socket::{tcp, udp, AnySocket},
    time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr},
};

use spin::Mutex;

pub static NET_INTERFACE: NetInterface = NetInterface::new();

pub fn init() {
    NET_INTERFACE.init();
}

pub struct NetInterface<'a> {
    inner: Mutex<Option<NetInterfaceInner<'a>>>,
}

pub struct NetInterfaceInner<'a> {
    pub device: Loopback,
    pub iface: Interface,
    pub sockets: SocketSet<'a>,
}

impl<'a> NetInterfaceInner<'a> {
    fn new() -> Self {
        let mut device = Loopback::new(Medium::Ethernet);
        let iface = {
            let config = match device.capabilities().medium {
                Medium::Ethernet => {
                    Config::new(EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]).into())
                }
                Medium::Ip => Config::new(smoltcp::wire::HardwareAddress::Ip),
            };

            let mut iface = Interface::new(
                config,
                &mut device,
                Instant::from_millis(current_time_duration().as_millis() as i64),
            );
            iface.update_ip_addrs(|ip_addrs| {
                ip_addrs
                    .push(IpCidr::new(IpAddress::v4(127, 0, 0, 1), 8))
                    .unwrap();
                ip_addrs
                    .push(IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 1), 128))
                    .unwrap();
            });
            iface
        };
        Self {
            device,
            iface,
            sockets: SocketSet::new(vec![]),
        }
    }
}

impl<'a> NetInterface<'a> {
    pub fn init(&self) {
        self._init();
    }

    pub fn add_socket<T>(&self, socket: T) -> SocketHandle
    where
        T: AnySocket<'a>,
    {
        self._add_socket(socket)
    }

    pub fn _init(&self) {
        *self.inner.lock() = Some(NetInterfaceInner::new());
    }
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn _add_socket<T>(&self, socket: T) -> SocketHandle
    where
        T: AnySocket<'a>,
    {
        self.inner.lock().as_mut().unwrap().sockets.add(socket)
    }

    pub fn tcp_socket<T>(&self, handler: SocketHandle, f: impl FnOnce(&mut tcp::Socket) -> T) -> T {
        f(self
            .inner
            .lock()
            .as_mut()
            .unwrap()
            .sockets
            .get_mut::<tcp::Socket>(handler))
    }

    pub fn udp_socket<T>(&self, handler: SocketHandle, f: impl FnOnce(&mut udp::Socket) -> T) -> T {
        f(self
            .inner
            .lock()
            .as_mut()
            .unwrap()
            .sockets
            .get_mut::<udp::Socket>(handler))
    }

    pub fn inner_handler<T>(&self, f: impl FnOnce(&mut NetInterfaceInner<'a>) -> T) -> T {
        f(&mut self.inner.lock().as_mut().unwrap())
    }

    pub fn poll(&self) {
        self._poll()
    }
    pub fn _poll(&self) {
        log::debug!("[NetInterface::poll] poll...");
        self.inner_handler(|inner| {
            inner.iface.poll(
                Instant::from_millis(current_time_duration().as_millis() as i64),
                &mut inner.device,
                &mut inner.sockets,
            );
        });
    }
    pub fn remove(&self, handler: SocketHandle) {
        self._remove(handler)
    }
    pub fn _remove(&self, handler: SocketHandle) {
        self.inner_handler(|inner| {
            inner.sockets.remove(handler);
        });
    }
}
