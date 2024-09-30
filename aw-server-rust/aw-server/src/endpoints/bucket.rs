use jwt::{create_jwt, validate_jwt, Claims};
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use serde::Deserialize;
use std::collections::HashMap;

use gethostname::gethostname;
use rocket::serde::json::Json;

use chrono::DateTime;
use chrono::Utc;

use aw_models::BucketsExport;
use aw_models::Event;
use aw_models::TryVec;
use aw_models::{Bucket, PublicBucket};

use rocket::http::Status;
use rocket::State;

mod jwt;

use crate::endpoints::util::BucketsExportRocket;
use crate::endpoints::{HttpErrorJson, ServerState};

#[get("/<user_id>")]
pub fn buckets_get(
    state: &State<ServerState>,
    user_id: i32,
) -> Result<Json<Vec<Bucket>>, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    match datastore.get_buckets(user_id) {
        Ok(bucketlist) => Ok(Json(bucketlist.values().cloned().collect())),
        Err(err) => Err(err.into()),
    }
}

#[get("/<bucket_id>/info")]
pub fn bucket_get(
    bucket_id: i64,
    state: &State<ServerState>,
) -> Result<Json<Bucket>, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    match datastore.get_bucket(bucket_id) {
        Ok(bucket) => Ok(Json(bucket)),
        Err(e) => Err(e.into()),
    }
}

/// Create a new bucket
///
/// If hostname is "!local", the hostname and device_id will be set from the server info.
/// This is useful for watchers which are known/assumed to run locally but might not know their hostname (like aw-watcher-web).
#[derive(Deserialize, Clone)]
pub struct Token(String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Token {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Look for the "Authorization" header
        if let Some(token_header) = request.headers().get_one("Authorization") {
            if let Some(token) = token_header.strip_prefix("Bearer ") {
                return Outcome::Success(Token(token.to_string()));
            }
        }

        Outcome::Forward(Status::Unauthorized)
    }
}

#[post("/", data = "<message>", format = "application/json")]
pub fn bucket_new(
    message: Json<PublicBucket>,
    state: &State<ServerState>,
    token: Token,
) -> Result<Json<i64>, HttpErrorJson> {
    let sent_bucket = message.into_inner();
    let token_string = token.clone().0;
    let user_id = match validate_jwt(&token_string) {
        Ok(user_id) => user_id,
        Err(_) => -1,
    };
    if user_id == -1 {
        return Err(HttpErrorJson::new(
            Status::Forbidden,
            "Authentication is required".to_string(),
        ));
    }
    let bucket = Bucket {
        bid: sent_bucket.bid,
        _type: sent_bucket._type,
        created: sent_bucket.created,
        data: sent_bucket.data,
        metadata: sent_bucket.metadata,
        events: sent_bucket.events,
        last_updated: sent_bucket.last_updated,
        user_id: user_id,
    };
    let datastore = endpoints_get_lock!(state.datastore);
    let ret = datastore.create_bucket(&bucket);
    let result = match ret {
        Ok(id) => Ok(Json(id)),
        Err(err) => Err(HttpErrorJson::new(
            Status::InternalServerError,
            "Could not create bucket".to_string(),
        )),
    };
    return result;
}

#[get("/<bucket_id>/events?<start>&<end>&<limit>&<team_id>")]
pub fn bucket_events_get(
    bucket_id: i64,
    start: Option<String>,
    end: Option<String>,
    limit: Option<u64>,
    team_id: Option<i32>,
    state: &State<ServerState>,
) -> Result<Json<Vec<Event>>, HttpErrorJson> {
    let starttime: Option<DateTime<Utc>> = match start {
        Some(dt_str) => match DateTime::parse_from_rfc3339(&dt_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                let err_msg = format!(
                    "Failed to parse starttime, datetime needs to be in rfc3339 format: {e}"
                );
                warn!("{}", err_msg);
                return Err(HttpErrorJson::new(Status::BadRequest, err_msg));
            }
        },
        None => None,
    };
    let endtime: Option<DateTime<Utc>> = match end {
        Some(dt_str) => match DateTime::parse_from_rfc3339(&dt_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                let err_msg =
                    format!("Failed to parse endtime, datetime needs to be in rfc3339 format: {e}");
                warn!("{}", err_msg);
                return Err(HttpErrorJson::new(Status::BadRequest, err_msg));
            }
        },
        None => None,
    };
    let datastore = endpoints_get_lock!(state.datastore);
    let res = datastore.get_user_events(bucket_id, starttime, endtime, limit, team_id);
    match res {
        Ok(events) => Ok(Json(events)),
        Err(err) => Err(err.into()),
    }
}

// Needs unused parameter, otherwise there'll be a route collision
// See: https://api.rocket.rs/master/rocket/struct.Route.html#resolving-collisions
#[get("/<bucket_id>/events/<event_id>?<_unused..>")]
pub fn bucket_events_get_single(
    bucket_id: i64,
    event_id: i64,
    _unused: Option<u64>,
    state: &State<ServerState>,
) -> Result<Json<Event>, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    let res = datastore.get_event(bucket_id, event_id);
    match res {
        Ok(events) => Ok(Json(events)),
        Err(err) => Err(err.into()),
    }
}

#[post("/<bucket_id>/events", data = "<events>", format = "application/json")]
pub fn bucket_events_create(
    bucket_id: i64,
    events: Json<Vec<Event>>,
    state: &State<ServerState>,
) -> Result<Json<Vec<Event>>, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    let res = datastore.insert_events(bucket_id, &events);
    match res {
        Ok(events) => Ok(Json(events)),
        Err(err) => Err(err.into()),
    }
}

#[post(
    "/<bucket_id>/heartbeat?<pulsetime>",
    data = "<heartbeat_json>",
    format = "application/json"
)]
pub fn bucket_events_heartbeat(
    bucket_id: i64,
    heartbeat_json: Json<Event>,
    pulsetime: f64,
    state: &State<ServerState>,
) -> Result<Json<Event>, HttpErrorJson> {
    let heartbeat = heartbeat_json.into_inner();
    let datastore = endpoints_get_lock!(state.datastore);
    match datastore.heartbeat(bucket_id, heartbeat, pulsetime) {
        Ok(e) => Ok(Json(e)),
        Err(err) => Err(err.into()),
    }
}

#[get("/<bucket_id>/events/count")]
pub fn bucket_event_count(
    bucket_id: i64,
    state: &State<ServerState>,
) -> Result<Json<u64>, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    let res = datastore.get_event_count(bucket_id, None, None);
    match res {
        Ok(eventcount) => Ok(Json(eventcount as u64)),
        Err(err) => Err(err.into()),
    }
}

#[delete("/<bucket_id>/events/<event_id>")]
pub fn bucket_events_delete_by_id(
    bucket_id: i64,
    event_id: i64,
    state: &State<ServerState>,
) -> Result<(), HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    match datastore.delete_events_by_id(bucket_id, vec![event_id]) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

#[get("/<bucket_id>/export")]
pub fn bucket_export(
    bucket_id: i64,
    state: &State<ServerState>,
) -> Result<BucketsExportRocket, HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    let mut export = BucketsExport {
        buckets: HashMap::new(),
    };
    let mut bucket = match datastore.get_bucket(bucket_id) {
        Ok(bucket) => bucket,
        Err(err) => return Err(err.into()),
    };
    /* TODO: Replace expect with http error */
    let events = datastore
        .get_events(bucket_id, None, None, None)
        .expect("Failed to get events for bucket");
    bucket.events = Some(TryVec::new(events));
    export.buckets.insert(bucket_id.to_string(), bucket);

    Ok(export.into())
}

#[delete("/<bucket_id>")]
pub fn bucket_delete(bucket_id: i64, state: &State<ServerState>) -> Result<(), HttpErrorJson> {
    let datastore = endpoints_get_lock!(state.datastore);
    match datastore.delete_bucket(bucket_id) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
