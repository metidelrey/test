use chrono::DateTime;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use serde_json::value::Value;
use std::collections::HashMap;

use crate::Event;
use crate::TryVec;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Bucket {
    // #[serde(skip)]
    pub bid: i64,
    #[serde(rename = "type")] /* type is a reserved Rust keyword */ pub _type: String,
    pub created: Option<DateTime<Utc>>,
    #[serde(default)]
    pub data: Map<String, Value>,
    #[serde(default, skip_deserializing)]
    pub metadata: BucketMetadata,
    // Events should only be "Some" during import/export
    // It's using a TryVec to discard only the events which were failed to be serialized so only a
    // few events are being dropped during import instead of failing the whole import
    pub events: Option<TryVec<Event>>,
    pub last_updated: Option<DateTime<Utc>>, // TODO: Should probably be moved into metadata field
    pub user_id: i32
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct PublicBucket {
    #[serde(skip)]
    pub bid: i64,
    #[serde(rename = "type")] /* type is a reserved Rust keyword */ pub _type: String,
    pub created: Option<DateTime<Utc>>,
    #[serde(default)]
    pub data: Map<String, Value>,
    #[serde(default, skip_deserializing)]
    pub metadata: BucketMetadata,
    // Events should only be "Some" during import/export
    // It's using a TryVec to discard only the events which were failed to be serialized so only a
    // few events are being dropped during import instead of failing the whole import
    pub events: Option<TryVec<Event>>,
    pub last_updated: Option<DateTime<Utc>>
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
pub struct BucketMetadata {
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct BucketsExport {
    pub buckets: HashMap<String, Bucket>,
}

#[test]
fn test_bucket() {
    let b = Bucket {
        bid:1,
        _type: "type".to_string(),
        created: None,
        data: json_map! {},
        metadata: BucketMetadata::default(),
        events: None,
        last_updated: None,
        user_id: 1,
    };
    debug!("bucket: {:?}", b);
}
