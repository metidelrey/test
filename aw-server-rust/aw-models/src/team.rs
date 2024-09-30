use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Team {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub ownerId: i32,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]

pub struct TeamRequestModel {
    pub name: String,
    pub description: String,
    pub ownerId: i32,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]

pub struct TeamResponseModel {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Member {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub lastname: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct TeamDetailModel {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub members: Vec<Member>,
    pub apps: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]

pub struct TeamUserModel {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]

pub struct TeamConfiguration {
    pub id: i32,
    pub team_id: i32,
    pub apps: Vec<String>,
}
