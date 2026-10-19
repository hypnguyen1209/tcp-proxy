#![warn(rust_2018_idioms)]

use tokio::io::copy_bidirectional;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};

use std::env;
use std::error::Error;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/unix.sock".to_string());
    let listen_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "/tmp/unix-clone.sock".to_string());

    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let _ = fs::remove_file(&listen_addr).is_err();

    let listener = UnixListener::bind(&listen_addr)?;

    fs::set_permissions(listen_addr, Permissions::from_mode(0o777))?;

    while let Ok((mut inbound, _)) = listener.accept().await {
        let to_addr = server_addr.clone();
        tokio::spawn(async move {
            let mut outbound = match UnixStream::connect(to_addr).await {
                Err(e) => {
                    let _ = inbound.shutdown().await;
                    eprintln!("connect: {}", e);
                    return;
                }
                Ok(r) => r,
            };

            if let Err(e) = copy_bidirectional(&mut inbound, &mut outbound).await {
                eprintln!("transfer: {}", e);
            }
        });
    }

    Ok(())
}
