use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::TcpStream as Stream;
use std::str;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

pub struct TCPStream {
    is_enabled: Arc<Mutex<bool>>,
    backend: Option<Backend>,
}

pub struct Backend {
    sender: mpsc::Sender<String>,
}

impl TCPStream {
    pub fn new() -> Self {
        TCPStream {
            is_enabled: Arc::new(Mutex::new(false)),
            backend: None,
        }
    }

    pub async fn connect<D>(
        address: String,
        is_enabled: Arc<Mutex<bool>>,
        receiver: mpsc::Receiver<String>,
        on_message: D,
    ) where
        D: Fn(&str) + Send + Copy + 'static,
    {
        let mut has_connection = false;

        // connect to the server and listen for messages
        while !has_connection && *is_enabled.lock().unwrap() {
            if let Ok(mut stream) = Stream::connect(&address) {
                has_connection = true;

                // set read timeout so it doesn't just block forever
                // this then allows us to exit early if tcp is disabled
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                    .unwrap();

                let mut buffer = [0; 512];
                while *is_enabled.lock().unwrap() {
                    // read messages from the server
                    if let Ok(bytes_read) = stream.read(&mut buffer) {
                        if bytes_read == 0 {
                            has_connection = false;
                            break;
                        }

                        let message = str::from_utf8(&buffer[..bytes_read - 1]).unwrap();
                        on_message(message);
                    }

                    // listen to receiver updates and send them to the server
                    if let Ok(message) = receiver.try_recv() {
                        stream.write_all(message.as_bytes()).unwrap();
                    }
                }
            } else {
                // check if it has been disabled in the meantime
                if !*is_enabled.lock().unwrap() {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    fn disconnect(&self) {
        *self.is_enabled.lock().unwrap() = false;
    }

    pub fn send_message(&self, message: &str) {
        if let Some(backend) = &self.backend {
            if let Err(error) = backend.sender.send(message.to_string()) {
                eprintln!("[TCPStream] Failed to send message: {error}");
            }
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn TCPCreate() -> Box<TCPStream> {
    Box::new(TCPStream::new())
}

#[no_mangle]
pub extern "C" fn TCPFree(stream: Option<Box<TCPStream>>) {
    if let Some(stream) = stream {
        drop(stream);
    }
}

type OnMessage = unsafe extern "C" fn(data: *const i8);

#[no_mangle]
pub extern "C" fn TCPConnect(
    mut stream: Option<&mut TCPStream>,
    host: *const i8,
    port: u16,
    on_message: OnMessage,
) {
    if stream.is_none() {
        eprintln!("[TCPStream] Failed to connect to server: was given an invalid instance pointer");
        return;
    }

    let host_str: &CStr = unsafe { CStr::from_ptr(host) };
    let host: &str = str::from_utf8(host_str.to_bytes()).unwrap();
    let address = format!("{host}:{port}");

    // set enabled to true
    *stream.as_mut().unwrap().is_enabled.lock().unwrap() = true;

    // make copy of is_enabled to be used in the thread
    let is_enabled = Arc::clone(&stream.as_ref().unwrap().is_enabled);

    // message stream writer and receiver - to be used for sending messages to the server
    let (tx, rx) = mpsc::channel();
    stream.as_mut().unwrap().backend = Some(Backend { sender: tx });

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("[TCPStream] failed to create async runtime")
            .block_on(async move {
                TCPStream::connect(address, is_enabled, rx, move |message| {
                    match std::ffi::CString::new(message) {
                        Ok(message) => unsafe { on_message(message.as_ptr()) },
                        Err(error) => {
                            eprintln!("[TCPStream] Failed to convert message to string: {error}")
                        }
                    }
                })
                .await;
            });
    });
}

#[no_mangle]
pub extern "C" fn TCPDisconnect(stream: Option<Box<TCPStream>>) {
    match stream {
        Some(stream) => stream.disconnect(),
        None => eprintln!(
            "[TCPStream] Failed to disconnect from server: was given an invalid instance pointer"
        ),
    }
}

#[no_mangle]
pub extern "C" fn TCPSendMessage(stream: Option<&mut TCPStream>, message: *const i8) {
    if stream.is_none() {
        eprintln!("[TCPStream] Failed to connect to server: was given an invalid instance pointer");
        return;
    }

    let message_str: &CStr = unsafe { CStr::from_ptr(message) };
    let message: &str = str::from_utf8(message_str.to_bytes()).unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("[TCPStream] failed to create async runtime")
        .block_on(async move {
            stream.unwrap().send_message(message);
        });
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_stream_init() {
        let tcp_stream = TCPStream::new();
        assert_eq!(false, *tcp_stream.is_enabled.lock().unwrap());
    }

    #[test]
    fn test_tcp_stream_is_enabled() {
        let tcp_stream = TCPStream::new();

        let is_enabled = Arc::clone(&tcp_stream.is_enabled);
        let handler = std::thread::spawn(move || {
            *is_enabled.lock().unwrap() = true;
            // sleep for a bit to allow the other thread to read the value before thread is done
            std::thread::sleep(std::time::Duration::from_millis(500));
        });

        std::thread::sleep(std::time::Duration::from_millis(100)); // small delay to allow thread to spin up
        assert_eq!(true, *tcp_stream.is_enabled.lock().unwrap());
        handler.join().unwrap();
    }
}
