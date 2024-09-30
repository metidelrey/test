use jsonwebtoken::{
    decode, encode,
    errors::{self, Error, ErrorKind, Result},
    Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub userId: i32,
    pub exp: usize,
}

// Secret key for signing the tokens
pub const SECRET: &[u8] = b"secret_key"; // Use a more secure key in production

pub fn create_jwt(claims: &Claims) -> Result<String> {
    let encoding_key = EncodingKey::from_secret(SECRET);
    let token = encode(&Header::default(), claims, &encoding_key)?;
    Ok(token)
}

pub fn validate_jwt(token: &str) -> Result<i32> {
    let decoding_key = DecodingKey::from_secret(SECRET);
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims.userId)
}
