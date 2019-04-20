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
        if let Some(user) = self.inner.react(reaction) {
            let mut payloads = Vec::new();

            while let Some(bytes_res) = user.recv() {
                match bytes_res {
                    Ok(bytes) => {
                        payloads.push(bytes);
                    }

                    // Connection broke while receiving data
                    Err(()) => {
                        let user_id = user.token();
                        self.inner.remove(user_id);
                        return Reaction::Continue;
                    }
                }
            }

            while let Some(res) = user.write() {
                if res.is_err() {
                    let user_id = user.token();
                    self.inner.remove(user_id);
                    return Reaction::Continue;
                }
            }

            let mut closed_connections = Vec::new();

            for bytes in payloads {
                for (user_id, con) in self.inner.iter_mut() {
                    let bytes_mut = BytesMut::from(bytes.clone());
                    let bytes = con.encode(bytes_mut);
                    con.add_payload(bytes);
                    while let Some(res) = con.write() {
                        if res.is_err() {
                            closed_connections.push(*user_id);
                            break
                        }
                    }
                }
            }

            // Clean up closed connections
            for user_id in closed_connections {
                self.inner.remove(user_id);
            }
        }

        Reaction::Continue
    }
}
