use crate::helix;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::TcpStream as Stream;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
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
        mut receiver: mpsc::Receiver<String>,
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

    pub async fn send_message(&self, message: &str) {
        if let Some(backend) = &self.backend {
            if let Err(error) = backend.sender.send(message.to_string()).await {
                println!("[TCPListener] Failed to send message: {:?}", error);
            }
        }
    }
}

// MARK: - C API

type OnMessage = unsafe extern "C" fn(data: *const i8);

#[no_mangle]
pub extern "C" fn HLX_TCPConnect(host: *const i8, port: u16, on_message: OnMessage) {
    let host_str: &CStr = unsafe { CStr::from_ptr(host) };
    let host: &str = str::from_utf8(host_str.to_bytes()).unwrap();
    let address = format!("{}:{}", host, port);

    // set enabled to true
    *helix!().tcp_stream.is_enabled.lock().unwrap() = true;

    // make copy of is_enabled to be used in the thread
    let is_enabled = Arc::clone(&helix!().tcp_stream.is_enabled);

    // message stream writer and receiver - to be used for sending messages to the server
    let (tx, rx) = mpsc::channel::<String>(10);
    helix!().tcp_stream.backend = Some(Backend { sender: tx });

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("[TCPListener] failed to create async runtime")
            .block_on(async move {
                TCPStream::connect(address, is_enabled, rx, move |message| {
                    match std::ffi::CString::new(message) {
                        Ok(message) => unsafe { on_message(message.as_ptr()) },
                        Err(error) => println!(
                            "[TCPListener] Failed to convert message to string: {:?}",
                            error
                        ),
                    }
                })
                .await;
            });
    });
}

#[no_mangle]
pub extern "C" fn HLX_TCPDisconnect() {
    helix!().tcp_stream.disconnect();
}

#[no_mangle]
pub extern "C" fn HLX_TCPSendMessage(message: *const i8) {
    let message_str: &CStr = unsafe { CStr::from_ptr(message) };
    let message: &str = str::from_utf8(message_str.to_bytes()).unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("[TCPListener] failed to create async runtime")
        .block_on(async move {
            helix!().tcp_stream.send_message(message).await;
        });
}
