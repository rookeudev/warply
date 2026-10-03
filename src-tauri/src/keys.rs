use base64::{engine::general_purpose::STANDARD, Engine};
use x25519_dalek::{PublicKey, StaticSecret};

pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

pub fn generate() -> KeyPair {
    let secret = StaticSecret::random();
    let public = PublicKey::from(&secret);
    KeyPair {
        private_key: STANDARD.encode(secret.to_bytes()),
        public_key: STANDARD.encode(public.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_wireguard_key_format() {
        let first = generate();
        let second = generate();
        let private = STANDARD.decode(&first.private_key).expect("private base64");
        let public = STANDARD.decode(&first.public_key).expect("public base64");
        assert_eq!(private.len(), 32);
        assert_eq!(public.len(), 32);
        assert_ne!(first.private_key, second.private_key);
        assert_eq!(first.private_key.len(), 44);
        assert_eq!(first.public_key.len(), 44);
    }
}
