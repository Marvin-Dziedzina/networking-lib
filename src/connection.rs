use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Connection {
    address: SocketAddr,
    pub socket: Arc<Mutex<TcpStream>>,
    on_message: Arc<&'static fn(usize, Vec<u8>)>,
}
impl Connection {
    pub fn new(socket: TcpStream, on_message: &'static fn(usize, Vec<u8>)) -> Connection {
        Connection {
            address: socket.peer_addr().expect("Could not get peer address!"),
            socket: Arc::new(Mutex::new(socket)),
            on_message: Arc::new(on_message),
        }
    }

    pub fn start_listening(&mut self) {
        let mutex_socket = self.socket.clone();
        let arc_on_message = self.on_message.clone();

        thread::spawn(move || loop {
            let mut socket = match mutex_socket.lock() {
                Ok(socket) => socket,
                Err(e) => {
                    println!("Could not lock mutex! Error: {}", e);
                    continue;
                }
            };

            let mut buf = Vec::new();
            let bytes_amount = match socket.read_to_end(&mut buf) {
                Ok(bytes_amount) => bytes_amount,
                Err(e) => {
                    println!("Could not read bytes from stream! Error: {}", e);
                    continue;
                }
            };

            arc_on_message(bytes_amount, buf.to_vec());
        });
    }

    pub fn send(&mut self, data: &[u8]) {
        let mut socket = match self.socket.lock() {
            Ok(socket) => socket,
            Err(e) => panic!("Could not lock mutex! Error: {}", e),
        };

        let mut written_bytes = 0;
        while written_bytes < data.len() {
            written_bytes += socket.write(&data).unwrap();
        }
    }

    fn action_byte_to_enum(byte: u8) -> Actions {
        match byte {
            200 => Actions::Message,
            255 => Actions::CloseConnection,
            _ => Actions::Undefined,
        }
    }

    fn enum_to_action_byte(action: Actions) -> u8 {
        match action {
            Actions::Message => 200,
            Actions::CloseConnection => 255,
            _ => 0,
        }
    }
}

enum Actions {
    Message,
    CloseConnection,
    Undefined,
}
