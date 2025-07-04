use super::{AF_INET, AF_INET6};
use crate::utils::error::SyscallErr;
use crate::utils::error::SyscallRet;
use crate::utils::{error::GeneralRet, random::RNG};
use core::convert::TryInto;
use core::mem;
use core::slice;
use smoltcp::wire::{IpAddress, IpEndpoint, IpListenEndpoint, Ipv4Address, Ipv6Address};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct SocketAddrv4 {
    sin_port: [u8; 2],
    sin_addr: [u8; 4],
}

impl SocketAddrv4 {
    /// user check first
    pub fn new(buf: &[u8]) -> Self {
        let addr = Self {
            sin_port: buf[2..4].try_into().expect("ipv4 port len err"),
            sin_addr: buf[4..8].try_into().expect("ipv4 addr len err"),
        };
        log::info!("[SocketAddrv4::new] new addr: {:?}", addr);
        addr
    }
    pub fn fill(&self, addr_buf: &mut [u8], addrlen: usize) {
        self._fill(addr_buf, addrlen);
    }
    pub fn _fill(&self, addr_buf: &mut [u8], addrlen: usize) {
        addr_buf.fill(0);
        addr_buf[0..2].copy_from_slice(u16::to_ne_bytes(AF_INET).as_slice());
        addr_buf[2..4].copy_from_slice(self.sin_port.as_slice());
        addr_buf[4..8].copy_from_slice(self.sin_addr.as_slice());
        unsafe {
            *(addrlen as *mut u32) = 8;
        }
    }
}

impl From<IpEndpoint> for SocketAddrv4 {
    fn from(value: IpEndpoint) -> Self {
        
        Self {
            sin_port: value.port.to_be_bytes(),
            sin_addr: value
                .addr
                .as_bytes()
                .try_into()
                .expect("ipv4 addr len error"),
        }
    }
}

impl From<SocketAddrv4> for IpEndpoint {
    fn from(value: SocketAddrv4) -> Self {
        
        // big end
        let port = u16::from_be_bytes(value.sin_port);
        Self::new(IpAddress::Ipv4(Ipv4Address(value.sin_addr)), port)
    }
}

impl From<SocketAddrv4> for IpListenEndpoint {
    fn from(value: SocketAddrv4) -> Self {
        

        // big end
        let port = u16::from_be_bytes(value.sin_port);
        let addr = Ipv4Address(value.sin_addr);
        if addr.is_unspecified() {
            if port != 0 {
                IpListenEndpoint { addr: None, port }
            } else {
                IpListenEndpoint {
                    addr: None,
                    port: unsafe { RNG.positive_u32() } as u16,
                }
            }
        } else {
            IpListenEndpoint {
                addr: Some(IpAddress::Ipv4(addr)),
                port,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct SocketAddrv6 {
    sin6_port: [u8; 2],
    sin6_flowinfo: [u8; 4],
    sin6_addr: [u8; 16],
}

impl SocketAddrv6 {
    /// user check first
    pub fn new(buf: &[u8]) -> Self {
        
        log::debug!("[SocketAddrv6::new] buf: {:?}", buf);
        let addr = Self {
            sin6_port: buf[2..4].try_into().expect("ipv6 port len err"),
            sin6_flowinfo: buf[4..8].try_into().expect("ipv6 flowinfo len err"),
            sin6_addr: buf[8..24].try_into().expect("ipv6 addr len err"),
        };
        log::debug!("[SocketAddrv6::new] new addr: {:?}", addr);
        addr
    }
    pub fn fill(&self, addr_buf: &mut [u8], addrlen: usize) {
        self._fill(addr_buf, addrlen)
    }
    pub fn _fill(&self, addr_buf: &mut [u8], addrlen: usize) {
        addr_buf.fill(0);
        addr_buf[0..2].copy_from_slice(u16::to_ne_bytes(AF_INET6).as_slice());
        addr_buf[2..4].copy_from_slice(self.sin6_port.as_slice());
        addr_buf[4..8].copy_from_slice(self.sin6_flowinfo.as_slice());
        addr_buf[8..24].copy_from_slice(self.sin6_addr.as_slice());
        unsafe {
            *(addrlen as *mut u32) = 24;
        }
    }
}

impl From<IpEndpoint> for SocketAddrv6 {
    fn from(value: IpEndpoint) -> Self {
        
        Self {
            sin6_port: value.port.to_be_bytes(),
            sin6_flowinfo: [0 as u8; 4],
            sin6_addr: value
                .addr
                .as_bytes()
                .try_into()
                .expect("ipv6 addr len error"),
        }
    }
}

impl From<SocketAddrv6> for IpEndpoint {
    fn from(value: SocketAddrv6) -> Self {
        
        // big end
        let port = u16::from_be_bytes(value.sin6_port);
        Self::new(IpAddress::Ipv6(Ipv6Address(value.sin6_addr)), port)
    }
}

impl From<SocketAddrv6> for IpListenEndpoint {
    fn from(value: SocketAddrv6) -> Self {
        
        // big end
        let port = u16::from_be_bytes(value.sin6_port);
        let addr = Ipv6Address(value.sin6_addr);
        if addr.is_unspecified() {
            if port != 0 {
                IpListenEndpoint { addr: None, port }
            } else {
                IpListenEndpoint {
                    addr: None,
                    port: unsafe { RNG.positive_u32() as u16 },
                }
            }
        } else {
            IpListenEndpoint {
                addr: Some(IpAddress::Ipv6(addr)),
                port,
            }
        }
    }
}
pub fn to_endpoint(listen_endpoint: IpListenEndpoint) -> IpEndpoint {
    _to_endpoint(listen_endpoint)
}

#[allow(unused)]
pub fn endpoint(addr_buf: &[u8]) -> GeneralRet<IpEndpoint> {
    _endpoint(addr_buf)
}

pub fn _to_endpoint(listen_endpoint: IpListenEndpoint) -> IpEndpoint {
    
    let addr = if listen_endpoint.addr.is_none() {
        IpAddress::v4(127, 0, 0, 1)
    } else {
        listen_endpoint.addr.unwrap()
    };
    IpEndpoint::new(addr, listen_endpoint.port)
}

#[allow(unused)]
pub fn _endpoint(addr_buf: &[u8]) -> GeneralRet<IpEndpoint> {
    let listen_endpoint = listen_endpoint(addr_buf)?;
    let addr = if listen_endpoint.addr.is_none() {
        IpAddress::v4(127, 0, 0, 1)
    } else {
        listen_endpoint.addr.unwrap()
    };
    Ok(IpEndpoint::new(addr, listen_endpoint.port))
}
use crate::net::current_task;
use crate::mm::translated_refmut;
pub fn fill_with_endpoint(endpoint: IpEndpoint, addr: usize, addrlen: usize) -> SyscallRet {
    _fill_with_endpoint(endpoint, addr, addrlen)
}
pub fn listen_endpoint(addr_buf: &[u8]) -> GeneralRet<IpListenEndpoint> {
    _listen_endpoint(addr_buf)
}

pub fn _fill_with_endpoint(endpoint: IpEndpoint, addr: usize, addrlen: usize) -> SyscallRet {
    log::debug!(
        "[address::fill_with_endpoint] fill addr {} with endpoint {:?}",
        addr,
        endpoint
    );
    let task = current_task().unwrap();
    let token = task.get_user_token();
    let addr = translated_refmut(token, addr as *mut u8).unwrap();
    match endpoint.addr {
        IpAddress::Ipv4(_) => {
            let len = mem::size_of::<u16>() + mem::size_of::<SocketAddrv4>();
            let addr_buf = unsafe { slice::from_raw_parts_mut(addr as *mut u8, len) };
            SocketAddrv4::from(endpoint).fill(addr_buf, addrlen);
        }
        IpAddress::Ipv6(_) => {
            let len = mem::size_of::<u16>() + mem::size_of::<SocketAddrv6>();
            let addr_buf = unsafe { slice::from_raw_parts_mut(addr as *mut u8, len) };
            SocketAddrv6::from(endpoint).fill(addr_buf, addrlen);
        }
    }
    Ok(0)
}

pub fn _listen_endpoint(addr_buf: &[u8]) -> GeneralRet<IpListenEndpoint> {
    let family = u16::from_ne_bytes(addr_buf[0..2].try_into().expect("family size wrong"));
    log::info!("[address::listen_enpoint] addr family {}", family);
    match family {
        AF_INET => {
            let ipv4 = SocketAddrv4::new(addr_buf);
            Ok(IpListenEndpoint::from(ipv4))
        }
        AF_INET6 => {
            let ipv6 = SocketAddrv6::new(addr_buf);
            Ok(IpListenEndpoint::from(ipv6))
        }
        _ => return Err(SyscallErr::EINVAL),
    }
}
