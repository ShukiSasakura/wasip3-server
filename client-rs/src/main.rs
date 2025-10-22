use std::env;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), std::io::Error> {

    let (ip, port) = match get_arg() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Failed to get args");
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid arguments"));
        },
    };

    let distination_addr = format!("{}:{}", ip, port);

    let mut stream = match TcpStream::connect(distination_addr) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Faild to connect: {}", e);
            return Err(e)
        },
    };

    let msg: [u8; 256] = [1; 256];

    for i in 0..3 {
        stream.write_all(&msg)?;
        println!("send message {}", i);

        sleep(Duration::from_secs(3));

        let mut ack: [u8; 256] = [0; 256];
        stream.read(&mut ack)?;
        println!("receive ack");
    }

    Ok(())

}

fn get_arg () -> Result<(String, u16), ()> {
    let args: Vec<String> = env::args().collect();

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

    let ip = args[1].clone();
    let port: u16 = args[2].parse().unwrap();

    Ok((ip, port))
}

