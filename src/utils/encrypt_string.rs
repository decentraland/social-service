use ring::aead::*;
use ring::hmac::HMAC_SHA256;
use ring::pbkdf2::*;
use ring::rand::SystemRandom;

fn encrypt(str: String) -> String {
    // The password will be used to generate a key
    let password = b"nice password";

    // Usually the salt has some random data and something that relates to the user
    // like an username
    let salt = [0, 1, 2, 3, 4, 5, 6, 7];

    // Keys are sent as &[T] and must have 32 bytes
    let mut key = [0; 32];
    derive(&HMAC_SHA256, 100, &salt, &password[..], &mut key);

    // Ring uses the same input variable as output
    let mut in_out = str.clone();

    // The input/output variable need some space for a suffix
    println!("Tag len {}", CHACHA20_POLY1305.tag_len());
    for _ in 0..CHACHA20_POLY1305.tag_len() {
        in_out.push(0);
    }

    // Sealing key used to encrypt data
    let sealing_key = SealingKey::new(&CHACHA20_POLY1305, &key).unwrap();
}

#[cfg(test)]
mod tests {}
