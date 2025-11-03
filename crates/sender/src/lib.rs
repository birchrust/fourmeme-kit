use alloy::signers::local::PrivateKeySigner;
use anyhow::Error;

#[derive(Clone, Debug)]
pub struct Sender {
    pub singer: PrivateKeySigner,
}

impl Sender {
    #[inline]
    pub fn new(private_key: &str) -> Result<Self, Error> {
        let pk_signer: PrivateKeySigner = private_key.parse()?;
        Ok(Self { singer: pk_signer })
    }
}

#[test]
fn test_new() {
    let private_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let sender = Sender::new(private_key).unwrap();
    println!("Sender: {:?}", sender.singer.address());
}
