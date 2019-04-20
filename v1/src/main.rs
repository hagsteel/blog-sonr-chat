use sonr::errors::Result;
use sonr::net::stream::Stream;
use sonr::prelude::*;
use sonr_extras::{tcp_listener, LineCodec};

mod connections;
use connections::{Connections, UserConnection};

const BUFFER_SIZE: usize = 1024;

fn main() -> Result<()> {
    System::init()?;

    let listener = tcp_listener("0.0.0.0:4578")?.map(|s| {
        let stream = Stream::new(s).unwrap();
        UserConnection::new(stream, LineCodec::new(), BUFFER_SIZE, BUFFER_SIZE)
    });

    let connections = Connections::new();

    let run = listener.chain(connections);

    System::start(run)?;
    Ok(())
}
