use anyhow::Result;

#[allow(dead_code)]
pub fn test_niri() -> Result<()> {
    let mut n = Niri::new()?;
    n.print_workspaces()?;
    n.print_windows()?;

    Ok(())
}

struct Niri {
    socket: niri_ipc::socket::Socket,
}

impl Niri {
    pub fn new() -> Result<Self> {
        let socket = niri_ipc::socket::Socket::connect()
            .map_err(|e| anyhow::anyhow!("on Socket::connect(): {:?}", e))?;
        Ok(Self { socket })
    }

    pub fn print_windows(&mut self) -> Result<()> {
        let reply = self
            .socket
            .send(niri_ipc::Request::Windows)
            .map_err(|e| anyhow::anyhow!("on socket.send(): {:?}", e))?
            .map_err(|e| anyhow::anyhow!("on decoding Niri answer: {:?}", e))?;

        let niri_ipc::Response::Windows(windows) = reply else {
            return Err(anyhow::anyhow!("Unexpected response type from Niri"));
        };

        for window in &windows {
            println!("{window:?}");
        }

        Ok(())
    }

    pub fn print_workspaces(&mut self) -> Result<()> {
        let reply = self
            .socket
            .send(niri_ipc::Request::Workspaces)
            .map_err(|e| anyhow::anyhow!("on socket.send(): {:?}", e))?
            .map_err(|e| anyhow::anyhow!("on decoding Niri answer: {:?}", e))?;

        let niri_ipc::Response::Workspaces(workspaces) = reply else {
            return Err(anyhow::anyhow!("Unexpected response type from Niri"));
        };

        for workspace in &workspaces {
            println!("{workspace:?}");
        }

        Ok(())
    }
}
