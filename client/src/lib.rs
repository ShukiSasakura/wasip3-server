mod bindings {
    pub mod server;
}

use std::error::Error;
// use std::io::stdout;
use std::net::Ipv4Addr;
use std::thread::sleep;

use crate::bindings::server::wasi::sockets::types::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, TcpSocket,
};
use crate::bindings::server::{exports, wit_stream, wasi::cli::environment, wasi::cli::stdout};
use futures::join;
use wit_bindgen::{AbiBuffer, StreamReader, StreamResult};

struct Component;

bindings::server::export!(Component);

impl exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(),()> {
        let args = environment::get_arguments();

        if args.len() != 3 {
            return Err(());
        }

        // Validate if args[1] is a valid IPv4 address
        if args[1].parse::<std::net::Ipv4Addr>().is_err() {
            return Err(());
        }

        // Validate if args[2] is convertible to u16
        if args[2].parse::<u16>().is_err() {
            return Err(());
        }

        let ipaddress_string = args[1].clone();
        let port = args[2].parse::<u16>().map_err(|_| ())?;
        // println!("ipaddress={:?}, port={:?}", ipaddress, port);

        // ipaddress が 127.0.0.1 の形式なので split して address 引数に渡す
        let ipaddress: Ipv4Addr = ipaddress_string.parse().expect("invalid IPv4 address");
        let octets = ipaddress.octets();

        client(
            IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: port,                 // use any free port
                address: (octets[0], octets[1], octets[2], octets[3]),
            }),
        )
            .await

        // Ok(())
    }
}

async fn client(family: IpAddressFamily, connect_address: IpSocketAddress) -> Result<(),()> {
    let msg = "Hello, World!";

    let client = TcpSocket::create(family).unwrap();
    client.connect(connect_address).await.unwrap();
    println!("connect");
    let (mut data_tx, data_rx) = wit_stream::new();

    join!(
        async {
            client.send(data_rx).await.unwrap();
        },
        async {
            let buf = Vec::with_capacity(100);
            let (mut ack_rx, _fut) = client.receive();
            println!("send message");
            let remaining = data_tx.write_all(msg.into()).await;
            assert!(remaining.is_empty());
            println!("receive ack");
            let (r_result, r_data) = ack_rx.read(buf).await;
            println!("ack: {:?}", r_data);
            drop(data_tx);
        },
        async {
            sleep(std::time::Duration::from_secs(10));
        }
    );
    Ok(())
}

