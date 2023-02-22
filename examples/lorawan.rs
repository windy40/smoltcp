//! LoRaWAN exzmple
//!
//! This example is designed to run using the LWNSimulator as a network interface
//!
//!

mod utils;

use log::{info,debug, LevelFilter};
use env_logger::{Env, Builder};
// use std::os::unix::io::AsRawFd;
use std::str;

use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{LorawanInterface, Medium};
// use smoltcp::phy::wait as phy_wait;
// use smoltcp::socket::tcp;
use std::{thread, time};

use smoltcp::socket::udp;
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr};

// log init
fn configure_log() {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "info")
        .write_style_or("MY_LOG_STYLE", "always");

    Builder::from_env(env)
        .filter_module("lwnsim_api_rs", LevelFilter::Debug)
        .filter_module("smoltcp", LevelFilter::Trace)
        .filter_module("lorawan", LevelFilter::Debug)
        .init();
}
fn main() {
//    utils::setup_logging("info,lwnsim_api_rs=debug/");

    configure_log();

    let (mut opts, mut free) = utils::create_options();
    utils::add_middleware_options(&mut opts, &mut free);
    let mut matches = utils::parse_options(&opts, free);


    info!("[Lorawan]creating iface ");
    let device = LorawanInterface::new("devEUI", Medium::Lorawan).unwrap();
    //    let fd = device.as_raw_fd();

    let mut device =
        utils::parse_middleware_options(&mut matches, device, /*loopback=*/ false);



    // Create interface
    let /* mut */ config = Config::new();
    // config.random_seed = rand::random();
    // config.hardware_addr =
    //     Some(Ieee802154Address::Extended([0x1a, 0x0b, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42]).into());
    // config.pan_id = Some(Ieee802154Pan(0xbeef));


    let mut iface = Interface::new(config, &mut device);
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(
                IpAddress::v6(0xfe80, 0, 0, 0, 0x180b, 0x4242, 0x4242, 0x4242),
                64,
            ))
            .unwrap();
    });

    // Create sockets
    let udp_rx_buffer = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY], vec![0; 1280]);
    let udp_tx_buffer = udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY], vec![0; 1280]);
    let udp_socket = udp::Socket::new(udp_rx_buffer, udp_tx_buffer);

    // let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    // let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    // let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);

    let mut sockets = SocketSet::new(vec![]);
    let udp_handle = sockets.add(udp_socket);
    //    let tcp_handle = sockets.add(tcp_socket);

    // let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
    // socket.listen(50000).unwrap();

    // let mut tcp_active = false;
    let dur_1s = time::Duration::from_secs(1);

    loop {
        let timestamp = Instant::now();

        info!("[Lorawan][iface.poll]start");
        let poll_res = iface.poll(timestamp, &mut device, &mut sockets);
        info!("[Lorawan][iface.poll]{:?}",poll_res);

        // udp:6969: respond "hello"
        let socket = sockets.get_mut::<udp::Socket>(udp_handle);
        if !socket.is_open() {
            socket.bind(6969).unwrap() // meaning less for Lorawan
        }

        let payload = "Hello".to_string();
        let endpoint = smoltcp::wire::IpEndpoint {
            addr: IpAddress::v6(0xfe80, 0, 0, 0, 0x180b, 0x4242, 0x4242, 0x4242),
            port: 9999,
        };
        debug!("[Lorawan][udp]send data: {:?}", payload);
        socket.send_slice(payload.as_bytes(), endpoint).unwrap();

        let mut buffer = vec![0; 1500]; // check lorawan mtu and header length
        debug!("[Lorawan][udp socket]start recv");
        #[allow(unused_variables)]
        let client = match socket.recv() {
            Ok((data, endpoint)) => {
                debug!(
                    "[Lorawan][udp socket]recv data: {:?} from {}",
                    str::from_utf8(data).unwrap(),
                    endpoint
                );
                buffer[..data.len()].copy_from_slice(data);
                Some((data.len(), endpoint))
            }
            Err(_) => None,
        };
        // if let Some((len, endpoint)) = client {
        //     debug!(
        //         "udp (LoRaWAN) send data: {:?}",
        //         str::from_utf8(&buffer[..len]).unwrap()
        //     );
        //     socket.send_slice(&buffer[..len], endpoint).unwrap();
        // }

        // let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        // if socket.is_active() && !tcp_active {
        //     debug!("connected");
        // } else if !socket.is_active() && tcp_active {
        //     debug!("disconnected");
        // }
        // tcp_active = socket.is_active();

        // if socket.may_recv() {
        //     let data = socket
        //         .recv(|data| {
        //             let data = data.to_owned();
        //             if !data.is_empty() {
        //                 debug!(
        //                     "recv data: {:?}",
        //                     str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
        //                 );
        //             }
        //             (data.len(), data)
        //         })
        //         .unwrap();

        //     if socket.can_send() && !data.is_empty() {
        //         debug!(
        //             "send data: {:?}",
        //             str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
        //         );
        //         socket.send_slice(&data[..]).unwrap();
        //     }
        // } else if socket.may_send() {
        //     debug!("close");
        //     socket.close();
        // }

        //        phy_wait(fd, iface.poll_delay(timestamp, &sockets)).expect("wait error");

        thread::sleep(dur_1s);
    }
}
