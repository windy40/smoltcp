use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

use std::str;

use std::io;
// use std::os::unix::io::{AsRawFd, RawFd};



use crate::phy::{self,Device, DeviceCapabilities, Medium};
use lwnsim_api_rs::{LWNSIM, LORA, AF_LORA, SOCK_RAW, Socket, OTAA, LwnsimError, CmdErrorKind};
use base64::{engine::general_purpose, Engine as _};

use log::{info, error, debug};
use std::{thread, time};

use crate::time::Instant;

//static URL: &str = "http://localhost:8000";
static URL: &str = "http://172.24.80.1:8000";
static DEV_EUI: &str = "359ac7cd01bc8aff";
static APP_KEY: &str = "f1c4081b61e9bee79bef58b5347e78a5"; // set as device info in LWNSim
static JOIN_EUI: &str = "0000000000000000"; // set as device info in LWNSim

/// A  LoRaWAN interface.
#[derive(Debug)]
pub struct LorawanInterface {
    lower: Rc<RefCell<Socket>>,
    mtu: usize,
    medium: Medium,
}

/* impl AsRawFd for LorawanInterface {
    fn as_raw_fd(&self) -> RawFd {
        self.lower.borrow().as_raw_fd()
    }
} */

impl LorawanInterface {
    /// Attaches to a simulated LoRaWAN interface 
    /// ?? identified by 'name' a string with devEUI
    ///
    pub fn new(name: &str, medium: Medium) -> io::Result<LorawanInterface> {
        let mut lower = Socket::new(AF_LORA, SOCK_RAW);

//        lower.attach_interface()?;
        let dur_1s = time::Duration::from_secs(1);
        info!("[phy]connecting to LWNSimulator");
        LWNSIM.lock().unwrap().connect(URL, DEV_EUI);
        thread::sleep(dur_1s);
        //   lora=LoRa.LoRa( mode=LoRa.LORAWAN, region=LoRa.EU868, log_enable=True)
        info!("phy iface]linking to dev {:?}", DEV_EUI);
        LORA.lock().unwrap().activate().map_err(|e| {error!("Could not activate device (error : {:?})",e); /* LWNSIM.lock().unwrap().disconnect(); */ return 1;});   
        thread::sleep(dur_1s);
        // lora.join(...)
        // create an OTAA authentication parameters
        //app_eui = binascii.unhexlify('0000000000000000'.replace(' ',''))
        //app_key = binascii.unhexlify('2CC172969D5CC26382E0AD054568CE3E'.replace(' ',''))
        //app_key = binascii.unhexlify(''.replace(' ',''))
        info!("[phy]start joining device");
        LORA.lock().unwrap().join(
            OTAA,
            (JOIN_EUI.to_string(), APP_KEY.to_string()),// not used 
            Some(0),  // not used 
            Some(0), // not used LWNSim manages DR depending on device info
        );
    
        while !LORA.lock().unwrap().has_joined() {
            thread::sleep(dur_1s);
            debug!("[phy]not yet joined...");
        }
        thread::sleep(dur_1s);
        info!("[phy]device joined...");




        let mtu = 1000 ; // lorawan mtu = ?
        Ok(LorawanInterface {
            lower: Rc::new(RefCell::new(lower)),
            mtu,
            medium,
        })
    }
}

impl Device for LorawanInterface {
    type RxToken<'a> = RxToken;
    type TxToken<'a> = TxToken;

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            max_transmission_unit: self.mtu,
            medium: self.medium,
            ..DeviceCapabilities::default()
        }
    }

//to be adapted to lwnsim-api
    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut lower = self.lower.borrow_mut();

        //receive
/*         
        let mut buffer = vec![0; self.mtu];
        match lower.recv(&mut buffer[..]) {
            Ok(size) => {
                buffer.resize(size, 0);
                let rx = RxToken { buffer };
                let tx = TxToken {
                    lower: self.lower.clone(),
                };
                Some((rx, tx))
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => None,
            Err(err) => panic!("{}", err),
        } */
        lower.setblocking(false);
        match lower.recv(self.mtu){
            Ok(s)=> {
                let size = s.len();
                net_trace!("[receive()]Ok {:?} (len= {:?})",s,size);
                if size > self.mtu {
                    panic!("[smoltcp::phy::lorawan_interface] recv String length greater than mtu");
                }

                let rx = RxToken { buffer : s.as_bytes().to_vec() };
                let tx = TxToken {
                    lower: self.lower.clone(),
                };
                Some((rx, tx))
            },
            Err(LwnsimError::CmdError(CmdErrorKind::NoDataDWrecv)) => {
                net_debug!("[receive()] No downlink data available");
                None},
            Err(e)=> panic!("{}", e),
         }
    }
//to be adapted to lwnsim-api
    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxToken {
            lower: self.lower.clone(),
        })
    }
}

#[doc(hidden)]
pub struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.buffer[..])
    }
}

#[doc(hidden)]
pub struct TxToken {
    lower: Rc<RefCell<Socket>>,
}

impl phy::TxToken for TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut lower = self.lower.borrow_mut();
        let mut buffer = vec![0; len];
        let result = f(&mut buffer);

        // convert buffer in a base64 string as lwnsim_api_rs send expects a String
        // 
        let data = general_purpose::STANDARD.encode(&buffer[..]);
    
        lower.setblocking(true);
        lower.settimeout(Some(3));
        match lower.send(data.as_str()) {
            Ok(_) => {net_trace!("[tx_token.consume()] send OK (packet {})",data.as_str());}
            Err(err) => panic!("{}", err),
        }
        result
    }
}
