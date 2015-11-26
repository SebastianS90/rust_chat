use std::io::*;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::thread;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;

static LISTEN: &'static str = "127.0.0.1:1337";

// Map for all connected clients containing their name and stream.
type UserMapValue = (String, TcpStream);
type UserMap = HashMap<SocketAddr, UserMapValue>;

fn main() {
	// Manage UserMap in mutex.
	let clients = Arc::new(Mutex::new(HashMap::new()));
	
	// Start a TCP listener.
	let listener = match TcpListener::bind(LISTEN) {
		Ok(listener) => listener,
		Err(e) => panic!("could not read start TCP listener: {}", e)
	};
	
	println!("You can connect to {}, e.g. with the 'nc' program.", LISTEN);
	
	// Accept connections and process them, spawning a new thread for each one.
	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let clients = clients.clone();
				thread::spawn(move|| {
					// connection succeeded
					handle_client(stream, clients)
				});
			}
			Err(e) => {
				let _ = writeln!(std::io::stderr(), "Connection failed: {}", e);
			}
		}
	}
	
	// close the socket server
	drop(listener);
}


fn handle_client(stream: TcpStream, clients: Arc<Mutex<UserMap>>) {
	// Get client IP and port
	let client = stream.peer_addr().unwrap();
	println!("New connection from {}", client);
	
	// Buffered reading and writing is more performant
	let mut reader = BufReader::new(&stream);
	let mut writer = BufWriter::new(&stream);
	
	// Write an entire line to the client.
	// Can fail on IO errors, due to try! macro.
	macro_rules! send {
		($line:expr) => ({
			try!(writeln!(writer, "{}", $line));
			try!(writer.flush());
		})
	}
	
	// Read an entire line from the client.
	// Can fail on IO errors or when EOF is reached.
	macro_rules! receive {
		() => ({
			let mut line = String::new();
			match reader.read_line(&mut line) {
				Ok(len) => {
					if len == 0 {
						// Reader is at EOF. Could use ErrorKind::UnexpectedEOF, but still unstable. 
						return Err(Error::new(ErrorKind::Other, "unexpected EOF"));
					}
					line.pop();
				}
				Err(e) => { return Err(e); }
			};
			line
		})
	}
	
	// Initialization: Ask user for his name.
	let name = match (|| {
		send!("Welcome to RustChat!");
		send!("Please give your name:");
		let name = receive!();
		println!("Client {} identified as {}", client, name);
		send!("You can now type messages. Leave this chat with 'exit' or 'quit'.");
		Ok(name)
	})() {
		Ok(name) => name,
		Err(e) => {
			println!("Client {} disappeared during initialization: {}", client, e);
			return ();
		}
	};
	
	// Add user to global map.
	{
		let mut lock = clients.lock().unwrap();
		(*lock).insert(client, (name.clone(), stream.try_clone().unwrap()));
		distriute_message(&format!("{} joined.", name), &client, &mut lock);
	}
	
	// Chat loop: Receive messages from user.
	match (|| {
		loop {
			let input = receive!();
			if input == "exit" || input == "quit" {
				send!("Bye!");
				return Ok(());
			}
			
			// Distribute message.
			println!("{} <{}>: {}", client, name, input);
			{
				let mut lock = clients.lock().unwrap();
				distriute_message(&format!("<{}>: {}", name, input), &client, &mut lock);
			}
		}
	})() {
		Ok(_) => {
			println!("Client {} <{}> left.", client, name);
		},
		Err(e) => {
			println!("Client {} <{}> disappeared during chat: {}", client, name, e);
		}
	}
	
	// Remove user from global map.
	{
		let mut lock = clients.lock().unwrap();
		disconnect_user(&name, &client, &mut lock);
	}
}

fn distriute_message(msg: &str, not_to: &SocketAddr, lock: &mut MutexGuard<UserMap>) {
	for (other_client, entry) in (*lock).iter() {
		if other_client != not_to {
			let other_name = &entry.0;
			let other_stream = &entry.1;
			match (|| -> Result<()> {
				let mut writer = BufWriter::new(other_stream);
				try!(writeln!(writer, "{}", msg));
				try!(writer.flush());
				return Ok(());
			})() {
				Ok(_) => { },
				Err(e) => {
					println!("Client {} <{}> disappeared during message distribution: {}", other_client, other_name, e);
					// TODO: can we somehow call disconnect_user here?
				}
			}	
		}
	}
}

fn disconnect_user(name: &str, client: &SocketAddr, lock: &mut MutexGuard<UserMap>) {
	(*lock).remove(&client);
	distriute_message(&format!("{} left", name), client, lock);
}