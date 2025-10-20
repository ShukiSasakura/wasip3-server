mod bindings {
    pub mod server;
}

// use std::error::Error;
// use std::io::stdout;
use std::net::Ipv4Addr;

use crate::bindings::server::wasi::sockets::types::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, TcpSocket,
};
use crate::bindings::server::{exports, wit_stream, wasi::cli::environment};
use futures::join;
use wit_bindgen::StreamResult;

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

        tcp_app(
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

async fn tcp_app(family: IpAddressFamily, bind_address: IpSocketAddress) -> Result<(),()> {
    let listener = TcpSocket::create(family).unwrap();

    // bind
    listener.bind(bind_address).unwrap();
    println!("bind:{:?}", bind_address);

    // accept
    listener.set_listen_backlog_size(32).unwrap();
    // TcpSocket を読み取るための StreamReader を作成
    let mut accept = listener.listen().unwrap();
    // let addr = listener.get_local_address().unwrap();

    loop {
        println!("wait accept");
        // 接続してきた TcpSocket を読み出す
        // TcpSocket はデータ送受信のための StreamReader を送受信できる
        wit_bindgen::yield_async().await;
        let sock = accept.next().await.unwrap();
        // wit_bindgen::yield_async().await;

        wit_bindgen::spawn(async move {
            // reveive rx for receiving data
            println!("sock receive");
            let (mut data_rx, _fut) = sock.receive();

            // send tx for sending ack
            let (mut ack_tx, ack_rx) = wit_stream::new();
            println!("join!");
            join!(
                async{
                    println!("send ack_rx");
                    let res = sock.send(ack_rx).await;
                    println!("sock send result: {:?}", res);
                },
                async{
                    // start waiting message
                    loop {
                        let buf = Vec::with_capacity(100);
                        // receive message
                        println!("wait receive message");
                        let (r_result, r_data) = data_rx.read(buf).await;
                        println!("r_result: {:?}", r_result);
                        if r_result == StreamResult::Dropped {
                            break;
                        }
                        // assert!(matches!(r_result, StreamResult::Complete(_)));
                        println!("read data: {:?}", r_data);

                        // send ack
                        println!("send ack");
                        let (s_result, buffer) = ack_tx.write(r_data.into()).await;
                        assert!(matches!(s_result, StreamResult::Complete(_)));
                        println!("s_result: {:?}", s_result);
                    }
                }
            );

        });
    }
}

