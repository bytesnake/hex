extern crate tokio;                                                                           
#[macro_use]
extern crate futures;
extern crate bytes;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

mod gossip;
mod discover;
mod local_ip;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
