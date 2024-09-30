use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: String,
    pub name: String,
    pub lastname: String,
    pub role: i8,
    pub password: String,
}
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]

pub struct PublicUser {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub lastname: String,
    pub role: i8,
}
