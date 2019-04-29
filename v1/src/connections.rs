use sonr::net::stream::StreamRef;
use sonr::reactor::{Reaction, Reactor};
use sonr_extras::Connection;
use sonr_extras::Connections as InnerConnections;
use sonr_extras::LineCodec;
use bytes::BytesMut;


pub type UserConnection<T> = Connection<T, LineCodec>;

pub struct Connections<T: StreamRef> {
    inner: InnerConnections<UserConnection<T>>,
}

impl<T: StreamRef> Connections<T> {
    pub fn new() -> Self {
        Self {
            inner: InnerConnections::new(),
        }
    }
}

impl<T: StreamRef> Reactor for Connections<T> {
    type Input = UserConnection<T>;
    type Output = ();

    fn react(&mut self, reaction: Reaction<Self::Input>) -> Reaction<()> {
        use Reaction::*;

        match self.inner.inner_react(reaction) {
            Continue => Continue,
            Event(ev) => ev.into(),
            Value(user) => {
                // All payloads to send
                let mut payloads = Vec::new();

                // Read data from the connection
                while let Some(bytes_res) = user.recv() {
                    match bytes_res {
                        Ok(bytes) => {
                            // encode the bytes, which in this 
                            // instance just appends `\n`
                            let bytes_mut = BytesMut::from(bytes);
                            let bytes = user.encode(bytes_mut);

                            payloads.push(bytes)
                        }

                        // Connection broke while receiving data
                        Err(_) => {
                            let user_id = user.token();
                            self.inner.remove(user_id);
                            return Reaction::Continue;
                        }
                    }
                }

                // Track all connections that closes
                // during the write process
                let mut closed_connections = Vec::new();

                // Iterate over the payloads
                for bytes in payloads {
                    for (user_id, con) in self.inner.iter_mut() {
                        // Add the payload to each connection
                        con.add_payload(bytes.clone());

                        // ... and try to write the data
                        while let Some(res) = con.write() {
                            if res.is_err() {
                                closed_connections.push(*user_id);
                                break
                            }
                        }
                    }
                }

                // Remove closed connections
                for user_id in closed_connections {
                    self.inner.remove(user_id);
                }

                Reaction::Continue
            }
        }
    }
}
