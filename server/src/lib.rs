mod bindings {
    pub mod server;
}

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
    async fn run() -> Result<(), ()> {

        let (ip_string, port) =  match get_arg () {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Faild to get args");
                return Err(());
            },
        };

        // ipaddress が 127.0.0.1 の形式なので split して address 引数に渡す
        let ipaddress: Ipv4Addr = ip_string.parse().expect("invalid IPv4 address");
        let octets = ipaddress.octets();

        tcp_app(
            IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: port,                 // use any free port
                address: (octets[0], octets[1], octets[2], octets[3]),
            }),
        )
            .await

    }
}

fn get_arg () -> Result<(String, u16), ()> {
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

    let ip_string = args[1].clone();
    let port: u16 = args[2].parse().unwrap();

    Ok((ip_string, port))
}

async fn tcp_app(family: IpAddressFamily, bind_address: IpSocketAddress) -> Result<(),()> {
    let mut id = 0;

    let listener = TcpSocket::create(family).unwrap();

    // bind
    listener.bind(bind_address).unwrap();

    // accept
    listener.set_listen_backlog_size(32).unwrap();
    // TcpSocket を読み取るための StreamReader を作成
    let mut accept = listener.listen().unwrap();
    // let addr = listener.get_local_address().unwrap();

    loop {
        let client_id = id;
        id += 1;
        // 接続してきた TcpSocket を読み出す
        // TcpSocket はデータ送受信のための StreamReader を送受信できる
        wit_bindgen::yield_async().await;
        let sock = accept.next().await.unwrap();
        // wit_bindgen::yield_async().await;

        wit_bindgen::spawn(async move {
            // reveive rx for receiving data
            let (mut data_rx, _fut) = sock.receive();

            // send tx for sending ack
            let (mut ack_tx, ack_rx) = wit_stream::new();
            join!(
                async{
                    let _res = sock.send(ack_rx).await;
                },
                async{
                    // start waiting message
                    loop {
                        let buf = Vec::with_capacity(256);
                        // receive message
                        let (r_result, r_data) = data_rx.read(buf).await;
                        if r_result == StreamResult::Dropped {
                            break;
                        }
                        println!("read message from client {}", client_id);

                        // send ack
                        let (s_result, _buffer) = ack_tx.write(r_data.into()).await;
                        if s_result == StreamResult::Dropped {
                            break;
                        }
                    }
                }
            );

        });
    }
}

