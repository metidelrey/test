use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn generate_hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    return password_hash;
}

pub fn verify_password(password: &str, hashed_password: &str) -> bool {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&hashed_password).expect("Error");
    return argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok();
}
