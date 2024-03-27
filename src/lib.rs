use std::fs::read;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    sync::{Arc, Mutex},
    thread,
};

mod connection;
mod states;
mod traits;

pub struct Server {
    ip: IpAddr,
    port: u16,
    max_connections: Arc<Option<usize>>,

    listener: Arc<TcpListener>,
    addresses: Arc<Mutex<Vec<SocketAddr>>>,
    connections: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,

    on_connect: Option<fn(SocketAddr)>,
    on_disconnect: Option<fn(SocketAddr)>,
    on_message: Option<fn(SocketAddr, String)>,
}
impl Server {
    /// Create a new instance of the Server struct
    ///
    /// ip: "xxx.xxx.xxx.xxx"; xxx: 0..255;
    ///
    /// set max_connections to None to disabling connection limit
    pub fn new(
        ip: Option<IpAddr>,
        port: Option<u16>,
        max_connections: Option<usize>,

        on_connect: Option<fn(SocketAddr)>,
        on_disconnect: Option<fn(SocketAddr)>,
        on_message: Option<fn(SocketAddr, String)>,
    ) -> Arc<Mutex<Server>> {
        let ip = match ip {
            Some(ip) => ip,
            None => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        };

        let port = match port {
            Some(port) => port,
            None => match (4000..30000).find(|port| Server::is_port_available(*port)) {
                Some(port) => port,
                None => panic!("Cant find any free port!"),
            },
        };

        let listener = TcpListener::bind((ip, port)).expect("Could not bind to port!");

        Arc::new(Mutex::new(Server {
            ip,
            port,
            max_connections: Arc::new(max_connections),

            listener: Arc::new(listener),
            addresses: Arc::new(Mutex::new(Vec::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),

            on_connect,
            on_disconnect,
            on_message,
        }))
    }

    pub fn start(&self) {
        println!("Starting on {}:{}...", &self.ip, &self.port);

        let listener_c = self.listener.clone();
        let max_connections_c = self.max_connections.clone();
        let addresses_c = self.addresses.clone();
        let connections_c = self.connections.clone();

        let on_connect = self.on_connect.clone();
        let on_message = self.on_message.clone();
        thread::spawn(move || loop {
            let (socket, address) = match listener_c.accept() {
                Ok((socket, address)) => (socket, address),
                Err(e) => {
                    println!("Could not establish connection! Error: {}", e);
                    continue;
                }
            };

            let mut addresses_lock = addresses_c.lock().expect("Mutex is poisoned!");
            let mut connections_lock = connections_c.lock().expect("Mutex is poisoned!");

            addresses_lock.push(address);
            connections_lock.insert(
                address,
                socket.try_clone().expect("Could not clone stream!"),
            );

            thread::spawn(move || {
                // Call on_connect in a extra thread if existent
                match on_connect {
                    Some(on_connect) => {
                        thread::spawn(move || on_connect(address.clone()));
                    }
                    None => (),
                };

                Server::handle_client(address, socket, on_message);
            });
        });

        println!("Listening...");
    }

    fn handle_client(
        address: SocketAddr,
        socket: TcpStream,
        on_message: Option<fn(SocketAddr, String)>,
    ) {
        println!("Got connection from {}", address.ip());

        loop {
            let mut reader = BufReader::new(&socket);

            let mut buf = Vec::new();
            let read_bytes = reader.read_to_end(&mut buf).unwrap();

            if read_bytes == 0 {
                continue;
            }

            match on_message {
                Some(on_message) => {
                    thread::spawn(move || {
                        on_message(address, String::from_utf8_lossy(&buf).to_string())
                    });
                }
                None => continue,
            }
        }
    }

    fn is_port_available(port: u16) -> bool {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

pub struct Client {
    ip: IpAddr,
    port: u16,
    socket: Option<TcpStream>,
}
impl Client {
    pub fn new(ip: Option<IpAddr>, port: u16) -> Client {
        let ip = match ip {
            Some(ip) => ip,
            None => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        };

        Client {
            ip,
            port,
            socket: Option::None,
        }
    }

    pub fn connect(&self) {
        let socket = TcpStream::connect((self.ip, self.port));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        time,
    };

    use super::*;

    #[test]
    fn server_test() {
        fn on_connect(address: SocketAddr) {
            println!("On Connect!");
        }

        fn on_disconnect(address: SocketAddr) {
            println!("{} disconnected!", address.to_string())
        }

        fn on_message(address: SocketAddr, message: String) {
            println!("[{}] {}", address.ip(), message);
        }

        let server = Server::new(
            Option::None,
            Option::Some(8989),
            Option::None,
            Option::Some(on_connect),
            Option::Some(on_disconnect),
            Option::Some(on_message),
        );
        let lock_server = server.lock().expect("Mutex is poisoned!");
        lock_server.start();

        let mut client_socket = TcpStream::connect("127.0.0.1:8989").unwrap();
        let mut writer = BufWriter::new(client_socket.try_clone().unwrap());
        writer.write(b"Test Msg!").unwrap();
        writer.flush().unwrap();
        drop(writer);

        thread::sleep(time::Duration::from_secs_f32(3.2))
    }
}
