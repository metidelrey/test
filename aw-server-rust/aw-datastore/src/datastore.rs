use aw_models::Bucket;
use aw_models::BucketMetadata;
use aw_models::Event;
use aw_models::Member;
use aw_models::PublicUser;
use aw_models::Team;
use aw_models::TeamConfiguration;
use aw_models::TeamRequestModel;
use aw_models::TeamUserModel;
use aw_models::User;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use rusqlite::Connection;
use serde_json::value::Value;
use std::collections::HashMap;

use rusqlite::params;
use rusqlite::types::ToSql;

use super::DatastoreError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn _get_db_version(conn: &Connection) -> i32 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

pub fn generate_hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    return password_hash;
}

/*
 * ### Database version changelog ###
 * 0: Uninitialized database
 * 1: Initialized database
 * 2: Added 'data' field to 'buckets' table
 * 3: see: https://github.com/ActivityWatch/aw-server-rust/pull/52
 * 4: Added 'key_value' table for storing key - value pairs
 */
static NEWEST_DB_VERSION: i32 = 5;

fn _create_tables(conn: &Connection, version: i32) -> bool {
    let mut first_init = false;

    if version < 1 {
        first_init = true;
        _migrate_v0_to_v1(conn);
    }

    if version < 2 {
        _migrate_v1_to_v2(conn);
    }

    if version < 3 {
        _migrate_v2_to_v3(conn);
    }

    if version < 4 {
        _migrate_v3_to_v4(conn);
    }
    if version < 5 {
        _migrate_new_version(conn);
    }
    first_init
}

fn _migrate_v0_to_v1(conn: &Connection) {
    /* Set up bucket table */
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS buckets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            created TEXT NOT NULL
        )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create buckets table");

    /* Set up index for bucket table */
    conn.execute(
        "CREATE INDEX IF NOT EXISTS bucket_id_index ON buckets(id)",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create buckets index");

    /* Set up events table */
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bucketrow INTEGER NOT NULL,
            starttime INTEGER NOT NULL,
            endtime INTEGER NOT NULL,
            data TEXT NOT NULL,
            FOREIGN KEY (bucketrow) REFERENCES buckets(id)
        )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create events table");

    /* Set up index for events table */
    conn.execute(
        "CREATE INDEX IF NOT EXISTS events_bucketrow_index ON events(bucketrow)",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create events_bucketrow index");
    conn.execute(
        "CREATE INDEX IF NOT EXISTS events_starttime_index ON events(starttime)",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create events_starttime index");
    conn.execute(
        "CREATE INDEX IF NOT EXISTS events_endtime_index ON events(endtime)",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create events_endtime index");
    /* Update database version */
    conn.pragma_update(None, "user_version", 1)
        .expect("Failed to update database version!");
}

fn _migrate_v1_to_v2(conn: &Connection) {
    info!("Upgrading database to v2, adding data field to buckets");
    conn.execute(
        "ALTER TABLE buckets ADD COLUMN data TEXT DEFAULT '{}';",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to upgrade database when adding data field to buckets");

    conn.pragma_update(None, "user_version", 2)
        .expect("Failed to update database version!");
}

fn _migrate_v2_to_v3(conn: &Connection) {
    // For details about why this migration was necessary, see: https://github.com/ActivityWatch/aw-server-rust/pull/52
    info!("Upgrading database to v3, replacing the broken data field for buckets");

    // Rename column, marking it as deprecated
    match conn.execute(
        "ALTER TABLE buckets RENAME COLUMN data TO data_deprecated;",
        &[] as &[&dyn ToSql],
    ) {
        Ok(_) => (),
        // This error is okay, it still has the intended effects
        Err(rusqlite::Error::ExecuteReturnedResults) => (),
        Err(e) => panic!("Unexpected error: {e:?}"),
    };

    // Create new correct column
    conn.execute(
        "ALTER TABLE buckets ADD COLUMN data TEXT NOT NULL DEFAULT '{}';",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to upgrade database when adding new data field to buckets");

    conn.pragma_update(None, "user_version", 3)
        .expect("Failed to update database version!");
}

fn _migrate_v3_to_v4(conn: &Connection) {
    info!("Upgrading database to v4, adding table for key-value storage");
    conn.execute(
        "CREATE TABLE key_value (
        key TEXT PRIMARY KEY,
        value TEXT,
        last_modified NUMBER NOT NULL
    );",
        [],
    )
    .expect("Failed to upgrade db and add key-value storage table");

    conn.pragma_update(None, "user_version", 4)
        .expect("Failed to update database version!");
}

fn _migrate_new_version(conn: &Connection) {
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS Users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            lastname TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL,
            role INTEGER NOT NULL,
            password TEXT NOT NULL
        )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create User table");

    // Should force password change after first login
    conn.execute(
    "INSERT INTO Users (username, email, name, lastname, password , role) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params!["admin", "admin@admin.com","admin", "admin", generate_hash("admin"), "1"] as &[&dyn ToSql],
    )
    .expect("Failed to insert Admin user");

    conn.execute(
        "
    CREATE TABLE IF NOT EXISTS Teams (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT,
        ownerId INTEGER NOT NULL,
        FOREIGN KEY (ownerId) REFERENCES Users(id)
    )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create Teams table");

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS TeamsUsers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            teamId INTEGER NOT NULL,
            userId INTEGER NOT NULL,
            FOREIGN KEY (teamId) REFERENCES Teams(id),
            FOREIGN KEY (userId) REFERENCES Users(id)
        )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create TeamsUsers table");

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS TeamConfiguration (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            teamId INTEGER NOT NULL,
            apps TEXT,
            FOREIGN KEY (teamId) REFERENCES Teams(id)
        )",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to create TeamConfiguration table");

    conn.execute(
        "ALTER TABLE buckets ADD COLUMN user_id INTEGER NOT NULL REFERENCES Users(id);",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to upgrade database when adding user_id field to buckets");

    conn.execute(
        "ALTER TABLE events ADD COLUMN team_id INTEGER NOT NULL;",
        &[] as &[&dyn ToSql],
    )
    .expect("Failed to upgrade database when adding team_id field to events");

    conn.pragma_update(None, "user_version", 5)
        .expect("Failed to update database version!");
}
pub struct DatastoreInstance {
    buckets_cache: HashMap<String, Bucket>,
    first_init: bool,
    pub db_version: i32,
}

impl DatastoreInstance {
    pub fn new(
        conn: &Connection,
        migrate_enabled: bool,
    ) -> Result<DatastoreInstance, DatastoreError> {
        let mut first_init = false;
        let db_version = _get_db_version(conn);

        if migrate_enabled {
            first_init = _create_tables(conn, db_version);
        } else if db_version < 0 {
            return Err(DatastoreError::Uninitialized(
                "Tried to open an uninitialized datastore with migration disabled".to_string(),
            ));
        } else if db_version != NEWEST_DB_VERSION {
            return Err(DatastoreError::OldDbVersion(format!(
                "\
                Tried to open an database with an incompatible database version!
                Database has version {db_version} while the supported version is {NEWEST_DB_VERSION}"
            )));
        }

        let mut ds = DatastoreInstance {
            buckets_cache: HashMap::new(),
            first_init,
            db_version,
        };
        ds.get_stored_buckets(conn)?;
        Ok(ds)
    }

    fn get_stored_buckets(&mut self, conn: &Connection) -> Result<(), DatastoreError> {
        let mut stmt = match conn.prepare(
            "
            SELECT  buckets.id, buckets.type, buckets.created,
                    min(events.starttime), max(events.endtime),
                    buckets.data, buckets.user_id
            FROM buckets
            LEFT OUTER JOIN events ON buckets.id = events.bucketrow
            GROUP BY buckets.id
            ;",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_stored_buckets SQL statement: {err}"
                )))
            }
        };
        let buckets = match stmt.query_map(&[] as &[&dyn ToSql], |row| {
            let opt_start_ns: Option<i64> = row.get(3)?;
            let opt_start = match opt_start_ns {
                Some(starttime_ns) => {
                    let seconds: i64 = starttime_ns / 1_000_000_000;
                    let subnanos: u32 = (starttime_ns % 1_000_000_000) as u32;
                    Some(DateTime::from_timestamp(seconds, subnanos).unwrap())
                }
                None => None,
            };

            let opt_end_ns: Option<i64> = row.get(4)?;
            let opt_end = match opt_end_ns {
                Some(endtime_ns) => {
                    let seconds: i64 = endtime_ns / 1_000_000_000;
                    let subnanos: u32 = (endtime_ns % 1_000_000_000) as u32;
                    Some(DateTime::from_timestamp(seconds, subnanos).unwrap())
                }
                None => None,
            };

            // If data column is not set (possible on old installations), use an empty map as default
            let data_str: String = row.get(5)?;
            let data_json = match serde_json::from_str(&data_str) {
                Ok(data) => data,
                Err(e) => {
                    return Err(rusqlite::Error::InvalidColumnName(format!(
                        "Failed to parse data to JSON: {e:?}"
                    )))
                }
            };

            Ok(Bucket {
                bid: row.get(0)?,
                _type: row.get(1)?,
                created: row.get(2)?,
                data: data_json,
                metadata: BucketMetadata {
                    start: opt_start,
                    end: opt_end,
                },
                events: None,
                last_updated: None,
                user_id: row.get(6)?,
            })
        }) {
            Ok(buckets) => buckets,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to query get_stored_buckets SQL statement: {err:?}"
                )))
            }
        };
        for bucket in buckets {
            match bucket {
                Ok(b) => {
                    self.buckets_cache.insert(b.bid.to_string(), b.clone());
                }
                Err(e) => {
                    return Err(DatastoreError::InternalError(format!(
                        "Failed to parse bucket from SQLite, database is corrupt! {e:?}"
                    )))
                }
            }
        }
        Ok(())
    }

    pub fn ensure_legacy_import(&mut self, conn: &Connection) -> Result<bool, ()> {
        use super::legacy_import::legacy_import;
        if !self.first_init {
            Ok(false)
        } else {
            self.first_init = false;
            match legacy_import(self, conn) {
                Ok(_) => {
                    info!("Successfully imported legacy database");
                    self.get_stored_buckets(conn).unwrap();
                    Ok(true)
                }
                Err(err) => {
                    warn!("Failed to import legacy database: {:?}", err);
                    Err(())
                }
            }
        }
    }

    pub fn create_bucket(
        &mut self,
        conn: &Connection,
        mut bucket: Bucket,
    ) -> Result<i64, DatastoreError> {
        bucket.created = match bucket.created {
            Some(created) => Some(created),
            None => Some(Utc::now()),
        };

        let previous_bucket_id: i64 = match self.get_bucket_from_database(conn, bucket.clone()) {
            Ok(bucket_id) => bucket_id,
            Err(_) => -1,
        };
        if previous_bucket_id != -1 {
            return Ok(previous_bucket_id);
        }

        let mut stmt = match conn.prepare(
            "
                INSERT INTO buckets (type, created, data, user_id)
                VALUES (?1, ?2, ?3, ?4)",
        ) {
            Ok(buckets) => buckets,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare create_bucket SQL statement: {err}"
                )))
            }
        };
        let data = serde_json::to_string(&bucket.data).unwrap();
        let res = stmt.execute([
            &bucket._type,
            &bucket.created as &dyn ToSql,
            &data,
            &bucket.user_id,
        ]);
        match res {
            Ok(_) => {
                info!("Created bucket {}", bucket.bid);
                // Get and set rowid
                let rowid: i64 = conn.last_insert_rowid();
                // Take out events from struct before caching
                let events = bucket.events;
                bucket.events = None;
                bucket.bid = rowid;
                // Cache bucket
                self.buckets_cache.insert(rowid.to_string(), bucket.clone());
                // Insert events
                if let Some(events) = events {
                    self.insert_events(conn, bucket.bid, events.take_inner())?;
                    bucket.events = None;
                }
                Ok(rowid)
            }
            // FIXME: This match is ugly, is it possible to write it in a cleaner way?
            Err(err) => match err {
                rusqlite::Error::SqliteFailure { 0: sqlerr, 1: _ } => match sqlerr.code {
                    rusqlite::ErrorCode::ConstraintViolation => {
                        Err(DatastoreError::BucketAlreadyExists(bucket.bid.to_string()))
                    }
                    _ => Err(DatastoreError::InternalError(format!(
                        "Failed to execute create_bucket SQL statement: {err}"
                    ))),
                },
                _ => Err(DatastoreError::InternalError(format!(
                    "Failed to execute create_bucket SQL statement: {err}"
                ))),
            },
        }
    }

    pub fn get_bucket_from_database(
        &mut self,
        conn: &Connection,
        bucket: Bucket,
    ) -> Result<i64, DatastoreError> {
        let mut stmt = match conn.prepare(
            "
                SELECT b.id FROM buckets b WHERE b.user_id=?1 and b.type=?2",
        ) {
            Ok(buckets) => buckets,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare create_bucket SQL statement: {err}"
                )))
            }
        };
        let bucket_id = match stmt.query_row([bucket.user_id.to_string(), bucket._type], |row| {
            Ok(row.get(0)?)
        }) {
            Ok(bucket_id) => bucket_id,
            Err(err) => -1,
        };
        Ok(bucket_id)
    }

    pub fn get_user_bucket_ids(
        &mut self,
        conn: &Connection,
        user_id:i32
    ) -> Result<Vec<i64>, DatastoreError> {
        let mut stmt = match conn.prepare(
            "
                SELECT b.id FROM buckets b WHERE b.user_id=?1",
        ) {
            Ok(buckets) => buckets,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare create_bucket SQL statement: {err}"
                )))
            }
        };
        let bucket_ids = match stmt.query_map([user_id.to_string()], |row| {
            Ok(row.get::<usize, i64>(0)?)
        }) {
            Ok(bucket_ids) => bucket_ids,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare create_bucket SQL statement: {err}"
                )))
            }
        };
        let mut ids: Vec<i64> = Vec::new();
        for id in bucket_ids.collect::<Vec<Result<i64, rusqlite::Error>>>(){
            match id {
                Ok(id) => ids.push(id),
                Err(_) => todo!("error")
            }
        }
        Ok(ids)
    }

    pub fn delete_bucket(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
    ) -> Result<(), DatastoreError> {
        let bucket = (self.get_bucket(bucket_id))?;
        // Delete all events in bucket
        match conn.execute("DELETE FROM events WHERE bucketrow = ?1", [&bucket.bid]) {
            Ok(_) => (),
            Err(err) => return Err(DatastoreError::InternalError(err.to_string())),
        }
        // Delete bucket itself
        match conn.execute("DELETE FROM buckets WHERE id = ?1", [&bucket.bid]) {
            Ok(_) => {
                self.buckets_cache.remove(&bucket_id.to_string());
                Ok(())
            }
            Err(err) => match err {
                rusqlite::Error::SqliteFailure { 0: sqlerr, 1: _ } => match sqlerr.code {
                    rusqlite::ErrorCode::ConstraintViolation => {
                        Err(DatastoreError::BucketAlreadyExists(bucket_id.to_string()))
                    }
                    _ => Err(DatastoreError::InternalError(err.to_string())),
                },
                _ => Err(DatastoreError::InternalError(err.to_string())),
            },
        }
    }

    pub fn get_bucket(&self, bucket_id: i64) -> Result<Bucket, DatastoreError> {
        let cached_bucket = self.buckets_cache.get(&bucket_id.to_string());
        match cached_bucket {
            Some(bucket) => Ok(bucket.clone()),
            None => Err(DatastoreError::NoSuchBucket(bucket_id.to_string())),
        }
    }

    pub fn get_buckets(&mut self, conn: &Connection, user_id: i32) -> HashMap<String, Bucket> {
        let user_bucket_ids = self.get_user_bucket_ids(conn, user_id).unwrap();
        let mut user_buckets: HashMap<String, Bucket> = HashMap::new();
        for id in user_bucket_ids{
            let bucket = self.buckets_cache.get(&id.to_string()).unwrap();
            user_buckets.insert(id.to_string(), bucket.clone());
        };
        user_buckets
    }

    pub fn insert_events(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        mut events: Vec<Event>,
    ) -> Result<Vec<Event>, DatastoreError> {
        let mut bucket = self.get_bucket(bucket_id)?;

        let mut stmt = match conn.prepare(
            "
                INSERT OR REPLACE INTO events(bucketrow, id, starttime, endtime, data, team_id)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare insert_events SQL statement: {err}"
                )))
            }
        };
        for event in &mut events {
            let starttime_nanos = event.timestamp.timestamp_nanos_opt().unwrap();
            let duration_nanos = match event.duration.num_nanoseconds() {
                Some(nanos) => nanos,
                None => {
                    return Err(DatastoreError::InternalError(
                        "Failed to convert duration to nanoseconds".to_string(),
                    ))
                }
            };
            let endtime_nanos = starttime_nanos + duration_nanos;
            let data = serde_json::to_string(&event.data).unwrap();
            let res = stmt.execute([
                &bucket.bid,
                &event.id as &dyn ToSql,
                &starttime_nanos,
                &endtime_nanos,
                &data as &dyn ToSql,
                &event.team_id,
            ]);
            match res {
                Ok(_) => {
                    self.update_endtime(&mut bucket, event);
                    let rowid = conn.last_insert_rowid();
                    event.id = Some(rowid);
                }
                Err(err) => {
                    return Err(DatastoreError::InternalError(format!(
                        "Failed to insert event: {event:?}, {err}"
                    )));
                }
            };
        }
        Ok(events)
    }

    pub fn delete_events_by_id(
        &self,
        conn: &Connection,
        bucket_id: i64,
        event_ids: Vec<i64>,
    ) -> Result<(), DatastoreError> {
        let bucket = self.get_bucket(bucket_id)?;
        let mut stmt = match conn.prepare(
            "
                DELETE FROM events
                WHERE bucketrow = ?1 AND id = ?2",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare insert_events SQL statement: {err}"
                )))
            }
        };
        for id in event_ids {
            let res = stmt.execute([&bucket.bid, &id as &dyn ToSql]);
            match res {
                Ok(_) => {}
                Err(err) => {
                    return Err(DatastoreError::InternalError(format!(
                        "Failed to delete event with id {id} in bucket {bucket_id}: {err:?}"
                    )));
                }
            };
        }
        Ok(())
    }

    // TODO: Function for deleting events by timerange with limit

    fn update_endtime(&mut self, bucket: &mut Bucket, event: &Event) {
        let mut update = false;
        /* Potentially update start */
        match bucket.metadata.start {
            None => {
                bucket.metadata.start = Some(event.timestamp);
                update = true;
            }
            Some(current_start) => {
                if current_start > event.timestamp {
                    bucket.metadata.start = Some(event.timestamp);
                    update = true;
                }
            }
        }
        /* Potentially update end */
        let event_endtime = event.calculate_endtime();
        match bucket.metadata.end {
            None => {
                bucket.metadata.end = Some(event_endtime);
                update = true;
            }
            Some(current_end) => {
                if current_end < event_endtime {
                    bucket.metadata.end = Some(event_endtime);
                    update = true;
                }
            }
        }
        /* Update buchets_cache if start or end has been updated */
        if update {
            self.buckets_cache
                .insert(bucket.bid.to_string(), bucket.clone());
        }
    }

    pub fn replace_last_event(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        event: &Event,
    ) -> Result<(), DatastoreError> {
        let mut bucket = self.get_bucket(bucket_id)?;

        let mut stmt = match conn.prepare(
            "
                UPDATE events
                SET starttime = ?2, endtime = ?3, data = ?4
                WHERE bucketrow = ?1
                    AND endtime = (SELECT max(endtime) FROM events WHERE bucketrow = ?1)
            ",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare replace_last_event SQL statement: {err}"
                )))
            }
        };
        let starttime_nanos = event.timestamp.timestamp_nanos_opt().unwrap();
        let duration_nanos = match event.duration.num_nanoseconds() {
            Some(nanos) => nanos,
            None => {
                return Err(DatastoreError::InternalError(
                    "Failed to convert duration to nanoseconds".to_string(),
                ))
            }
        };
        let endtime_nanos = starttime_nanos + duration_nanos;
        let data = serde_json::to_string(&event.data).unwrap();
        match stmt.execute([
            &bucket.bid,
            &starttime_nanos,
            &endtime_nanos,
            &data as &dyn ToSql,
        ]) {
            Ok(_) => self.update_endtime(&mut bucket, event),
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to execute replace_last_event SQL statement: {err}"
                )))
            }
        };
        Ok(())
    }

    pub fn heartbeat(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        heartbeat: Event,
        pulsetime: f64,
        last_heartbeat: &mut HashMap<String, Option<Event>>,
    ) -> Result<Event, DatastoreError> {
        self.get_bucket(bucket_id)?;
        if !last_heartbeat.contains_key(&bucket_id.to_string()) {
            last_heartbeat.insert(bucket_id.to_string(), None);
        }
        let last_event = match last_heartbeat.remove(&bucket_id.to_string()).unwrap() {
            // last heartbeat is in cache
            Some(last_event) => last_event,
            None => {
                // last heartbeat was not in cache, fetch from DB
                let mut last_event_vec = self.get_events(conn, bucket_id, None, None, Some(1))?;
                match last_event_vec.pop() {
                    Some(last_event) => last_event,
                    None => {
                        // There was no last event, insert and return
                        self.insert_events(conn, bucket_id, vec![heartbeat.clone()])?;
                        return Ok(heartbeat);
                    }
                }
            }
        };
        let inserted_heartbeat = match aw_transform::heartbeat(&last_event, &heartbeat, pulsetime) {
            Some(merged_heartbeat) => {
                debug!("Merged heartbeat successfully");
                self.replace_last_event(conn, bucket_id, &merged_heartbeat)?;
                merged_heartbeat
            }
            None => {
                debug!("Failed to merge heartbeat");
                self.insert_events(conn, bucket_id, vec![heartbeat.clone()])?;
                heartbeat
            }
        };
        last_heartbeat.insert(bucket_id.to_string(), Some(inserted_heartbeat.clone()));
        Ok(inserted_heartbeat)
    }

    pub fn get_event(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        event_id: i64,
    ) -> Result<Event, DatastoreError> {
        let bucket = self.get_bucket(bucket_id)?;

        let mut stmt = match conn.prepare(
            "
                SELECT id, starttime, endtime, data
                FROM events
                WHERE bucketrow = ?1
                    AND id = ?2
                LIMIT 1
            ;",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_event SQL statement: {err}"
                )))
            }
        };

        // TODO: Refactor to share row-parsing logic with get_events
        let row = match stmt.query_row([&bucket.bid, &event_id], |row| {
            let id = row.get(0)?;
            let starttime_ns: i64 = row.get(1)?;
            let endtime_ns: i64 = row.get(2)?;
            let data_str: String = row.get(3)?;

            let time_seconds: i64 = starttime_ns / 1_000_000_000;
            let time_subnanos: u32 = (starttime_ns % 1_000_000_000) as u32;
            let duration_ns = endtime_ns - starttime_ns;
            let data: serde_json::map::Map<String, Value> =
                serde_json::from_str(&data_str).unwrap();

            Ok(Event {
                id: Some(id),
                timestamp: DateTime::from_timestamp(time_seconds, time_subnanos).unwrap(),
                duration: Duration::nanoseconds(duration_ns),
                data,
                team_id: 1,
            })
        }) {
            Ok(rows) => rows,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to map get_event SQL statement: {err}"
                )))
            }
        };

        Ok(row)
    }

    pub fn get_events(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
        limit_opt: Option<u64>,
    ) -> Result<Vec<Event>, DatastoreError> {
        let bucket = self.get_bucket(bucket_id)?;

        let mut list = Vec::new();

        let starttime_filter_ns: i64 = match starttime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => 0,
        };
        let endtime_filter_ns: i64 = match endtime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => std::i64::MAX,
        };
        if starttime_filter_ns > endtime_filter_ns {
            warn!("Starttime in event query was lower than endtime!");
            return Ok(list);
        }
        let limit = match limit_opt {
            Some(l) => l as i64,
            None => -1,
        };

        let mut stmt = match conn.prepare(
            "
                SELECT id, starttime, endtime, data
                FROM events
                WHERE bucketrow = ?1
                    AND endtime >= ?2
                    AND starttime <= ?3
                ORDER BY starttime DESC
                LIMIT ?4
            ;",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_events SQL statement: {err}"
                )))
            }
        };

        let rows = match stmt.query_map(
            [
                &bucket.bid,
                &starttime_filter_ns,
                &endtime_filter_ns,
                &limit,
            ],
            |row| {
                let id = row.get(0)?;
                let mut starttime_ns: i64 = row.get(1)?;
                let mut endtime_ns: i64 = row.get(2)?;
                let data_str: String = row.get(3)?;

                if starttime_ns < starttime_filter_ns {
                    starttime_ns = starttime_filter_ns
                }
                if endtime_ns > endtime_filter_ns {
                    endtime_ns = endtime_filter_ns
                }
                let duration_ns = endtime_ns - starttime_ns;

                let time_seconds: i64 = starttime_ns / 1_000_000_000;
                let time_subnanos: u32 = (starttime_ns % 1_000_000_000) as u32;
                let data: serde_json::map::Map<String, Value> =
                    serde_json::from_str(&data_str).unwrap();

                Ok(Event {
                    id: Some(id),
                    timestamp: DateTime::from_timestamp(time_seconds, time_subnanos).unwrap(),
                    duration: Duration::nanoseconds(duration_ns),
                    data,
                    team_id: 1,
                })
            },
        ) {
            Ok(rows) => rows,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to map get_events SQL statement: {err}"
                )))
            }
        };
        for row in rows {
            match row {
                Ok(event) => list.push(event),
                Err(err) => warn!("Corrupt event in bucket {}: {}", bucket_id, err),
            };
        }

        Ok(list)
    }

    pub fn get_user_events(
        &mut self,
        conn: &Connection,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
        limit_opt: Option<u64>,
        team_id: Option<i32>
    ) -> Result<Vec<Event>, DatastoreError> {
        let bucket = self.get_bucket(bucket_id)?;

        let mut list = Vec::new();

        let starttime_filter_ns: i64 = match starttime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => 0,
        };
        let endtime_filter_ns: i64 = match endtime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => std::i64::MAX,
        };
        if starttime_filter_ns > endtime_filter_ns {
            warn!("Starttime in event query was lower than endtime!");
            return Ok(list);
        }
        let limit = match limit_opt {
            Some(l) => l as i64,
            None => -1,
        };

        let team_id = match team_id{
            Some(id) => id as i64,
            None => -1
        };

        let mut stmt = match conn.prepare(
            "
                SELECT id, starttime, endtime, data
                FROM events
                WHERE bucketrow = ?1
                    AND endtime >= ?2
                    AND starttime <= ?3
                    AND team_id = ?5
                ORDER BY starttime DESC
                LIMIT ?4
            ;",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_events SQL statement: {err}"
                )))
            }
        };

        let rows = match stmt.query_map(
            [
                &bucket.bid,
                &starttime_filter_ns,
                &endtime_filter_ns,
                &limit,
                &team_id
            ],
            |row| {
                let id = row.get(0)?;
                let mut starttime_ns: i64 = row.get(1)?;
                let mut endtime_ns: i64 = row.get(2)?;
                let data_str: String = row.get(3)?;

                if starttime_ns < starttime_filter_ns {
                    starttime_ns = starttime_filter_ns
                }
                if endtime_ns > endtime_filter_ns {
                    endtime_ns = endtime_filter_ns
                }
                let duration_ns = endtime_ns - starttime_ns;

                let time_seconds: i64 = starttime_ns / 1_000_000_000;
                let time_subnanos: u32 = (starttime_ns % 1_000_000_000) as u32;
                let data: serde_json::map::Map<String, Value> =
                    serde_json::from_str(&data_str).unwrap();

                Ok(Event {
                    id: Some(id),
                    timestamp: DateTime::from_timestamp(time_seconds, time_subnanos).unwrap(),
                    duration: Duration::nanoseconds(duration_ns),
                    data,
                    team_id: 1,
                })
            },
        ) {
            Ok(rows) => rows,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to map get_events SQL statement: {err}"
                )))
            }
        };
        for row in rows {
            match row {
                Ok(event) => list.push(event),
                Err(err) => warn!("Corrupt event in bucket {}: {}", bucket_id, err),
            };
        }

        Ok(list)
    }

    pub fn get_event_count(
        &self,
        conn: &Connection,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
    ) -> Result<i64, DatastoreError> {
        let bucket = self.get_bucket(bucket_id)?;

        let starttime_filter_ns: i64 = match starttime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => 0,
        };
        let endtime_filter_ns: i64 = match endtime_opt {
            Some(dt) => dt.timestamp_nanos_opt().unwrap(),
            None => std::i64::MAX,
        };
        if starttime_filter_ns >= endtime_filter_ns {
            warn!("Endtime in event query was same or lower than starttime!");
            return Ok(0);
        }

        let mut stmt = match conn.prepare(
            "
            SELECT count(*) FROM events
            WHERE bucketrow = ?1
                AND endtime >= ?2
                AND starttime <= ?3",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_event_count SQL statement: {err}",
                )))
            }
        };

        let count = match stmt.query_row(
            [&bucket.bid, &starttime_filter_ns, &endtime_filter_ns],
            |row| row.get(0),
        ) {
            Ok(count) => count,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to query get_event_count SQL statement: {err}"
                )))
            }
        };

        Ok(count)
    }

    pub fn insert_key_value(
        &self,
        conn: &Connection,
        key: &str,
        data: &str,
    ) -> Result<(), DatastoreError> {
        let mut stmt = match conn.prepare(
            "
                INSERT OR REPLACE INTO key_value(key, value, last_modified)
                VALUES (?1, ?2, ?3)",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare insert_value SQL statement: {err}"
                )))
            }
        };
        let timestamp = Utc::now().timestamp();
        #[allow(clippy::expect_fun_call)]
        stmt.execute(params![key, data, &timestamp])
            .expect(&format!("Failed to insert key-value pair: {key}"));
        Ok(())
    }

    pub fn delete_key_value(&self, conn: &Connection, key: &str) -> Result<(), DatastoreError> {
        conn.execute("DELETE FROM key_value WHERE key = ?1", [key])
            .expect("Error deleting value from database");
        Ok(())
    }

    pub fn get_key_value(&self, conn: &Connection, key: &str) -> Result<String, DatastoreError> {
        let mut stmt = match conn.prepare(
            "
                SELECT * FROM key_value WHERE KEY = ?1",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };

        match stmt.query_row([key], |row| row.get(1)) {
            Ok(result) => Ok(result),
            Err(err) => match err {
                rusqlite::Error::QueryReturnedNoRows => {
                    Err(DatastoreError::NoSuchKey(key.to_string()))
                }
                _ => Err(DatastoreError::InternalError(format!(
                    "Get value query failed for key {key}"
                ))),
            },
        }
    }

    pub fn get_key_values(
        &self,
        conn: &Connection,
        pattern: &str,
    ) -> Result<HashMap<String, String>, DatastoreError> {
        let mut stmt = match conn.prepare("SELECT key, value FROM key_value WHERE key LIKE ?") {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };

        let mut output = HashMap::<String, String>::new();
        // Rusqlite's get wants index and item type as parameters.
        let result = stmt.query_map([pattern], |row| {
            Ok((row.get::<usize, String>(0)?, row.get::<usize, String>(1)?))
        });
        match result {
            Ok(settings) => {
                for row in settings {
                    // Unwrap to String or panic on SQL row if type is invalid. Can't happen with a
                    // properly initialized table.
                    let (key, value) = row.unwrap();
                    // Only return keys starting with "settings.".
                    if !key.starts_with("settings.") {
                        continue;
                    }
                    output.insert(key, value);
                }
                Ok(output)
            }
            Err(err) => match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(output),
                _ => Err(DatastoreError::InternalError(
                    "Failed to get settings".to_string(),
                )),
            },
        }
    }

    pub fn get_user_by_email(
        &self,
        conn: &Connection,
        email: String,
    ) -> Result<User, DatastoreError> {
        let mut stmt = match conn.prepare(
            "SELECT id, username, email, name, lastname, role, password FROM Users WHERE email = ?1 LIMIT 1",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let user = match stmt.query_row([email.to_string()], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                name: row.get(3)?,
                lastname: row.get(4)?,
                role: row.get(5)?,
                password: row.get(6)?,
            })
        }) {
            Ok(rows) => rows,
            Err(err) => return Err(DatastoreError::NoUser()),
        };
        Ok(user)
    }

    pub fn get_user(&self, conn: &Connection, userId: i32) -> Result<PublicUser, DatastoreError> {
        let mut stmt = match conn
            .prepare("SELECT id, email, name, lastname, role FROM Users WHERE id = ?1 LIMIT 1")
        {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let user = match stmt.query_row([userId], |row| {
            Ok(PublicUser {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                lastname: row.get(3)?,
                role: row.get(4)?,
            })
        }) {
            Ok(rows) => rows,
            Err(err) => return Err(DatastoreError::NoUser()),
        };
        Ok(user)
    }

    pub fn signup(&self, conn: &Connection, user: User) -> Result<PublicUser, DatastoreError> {
        conn.execute(
            "INSERT INTO Users (email, name, lastname, password, role, username) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user.email, user.name, user.lastname, user.password, 2, user.username],
        )
        .expect("Could not insert");
        Ok(PublicUser {
            id: user.id,
            email: user.email,
            name: user.name,
            lastname: user.lastname,
            role: user.role,
        })
    }

    pub fn get_teams(&self, conn: &Connection, ownerId: i32) -> Result<Vec<Team>, DatastoreError> {
        let mut stmt = match conn.prepare("SELECT * FROM Teams WHERE ownerId = ?1") {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let rows = match stmt.query_map(params![ownerId], |row| {
            Ok(Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ownerId: row.get(3)?,
            })
        }) {
            Ok(teams) => teams,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let mut teams: Vec<Team> = Vec::new();
        for team in rows {
            match team {
                Ok(t) => teams.push(t),
                Err(err) => warn!("Bad data"),
            }
        }
        Ok(teams)
    }
    pub fn add_team(
        &self,
        conn: &Connection,
        team: TeamRequestModel,
        ownerId: i32,
    ) -> Result<Team, DatastoreError> {
        let mut stmt = match conn
            .prepare("INSERT INTO Teams (name,description,ownerId) VALUES (?1, ?2, ?3)")
        {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let team = match stmt.query_row([team.name, team.description, ownerId.to_string()], |row| {
            Ok(Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ownerId: row.get(3)?,
            })
        }) {
            Ok(rows) => rows,
            Err(err) => return Err(DatastoreError::NoUser()),
        };
        Ok(team)
    }

    pub fn get_team_members_count(
        &self,
        conn: &Connection,
        team_id: i32,
    ) -> Result<i64, DatastoreError> {
        let mut stmt = match conn.prepare("SELECT COUNT(*) FROM TeamsUsers WHERE teamId = ?1") {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };

        let count: i64 = match stmt.query_row([team_id], |row| Ok(row.get(0)?)) {
            Ok(i) => i,
            Err(err) => return Err(DatastoreError::NoUser()),
        };
        Ok(count)
    }

    pub fn get_team(&self, conn: &Connection, team_id: i32) -> Result<Team, DatastoreError> {
        let mut stmt = match conn.prepare("SELECT * FROM Teams WHERE id = ?1") {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };

        let team: Team = match stmt.query_row([team_id], |row| {
            Ok(Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ownerId: row.get(3)?,
            })
        }) {
            Ok(team) => team,
            Err(err) => return Err(DatastoreError::NoUser()),
        };
        Ok(team)
    }
    pub fn get_all_users(&self, conn: &Connection) -> Result<Vec<PublicUser>, DatastoreError> {
        let mut stmt = match conn
            .prepare("SELECT id, name, lastname, email, role FROM Users WHERE role = 2")
        {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let rows = match stmt.query_map(params![], |row| {
            Ok(PublicUser {
                id: row.get(0)?,
                name: row.get(1)?,
                lastname: row.get(2)?,
                email: row.get(3)?,
                role: row.get(4)?,
            })
        }) {
            Ok(users) => users,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let mut users: Vec<PublicUser> = Vec::new();
        for user in rows {
            match user {
                Ok(u) => users.push(u),
                Err(err) => warn!("Bad data"),
            }
        }
        Ok(users)
    }

    pub fn get_team_members(
        &self,
        conn: &Connection,
        team_id: i32,
    ) -> Result<Vec<Member>, DatastoreError> {
        let mut stmt = match conn.prepare(
            "SELECT tu.id as id, u.id as userId, u.name, u.lastname, u.email FROM TeamsUsers tu
        INNER Join Users u on tu.userId = u.id
        where tu.teamId=?1
        ",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let rows = match stmt.query_map(params![team_id], |row| {
            Ok(Member {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                lastname: row.get(3)?,
                email: row.get(4)?,
            })
        }) {
            Ok(members) => members,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let mut members: Vec<Member> = Vec::new();
        for team in rows {
            match team {
                Ok(t) => members.push(t),
                Err(err) => warn!("Bad data"),
            }
        }
        Ok(members)
    }

    pub fn add_members(
        &self,
        conn: &Connection,
        team_id: i32,
        members: Vec<i32>,
    ) -> Result<bool, DatastoreError> {
        let mut stmt = match conn.prepare("INSERT INTO TeamsUsers (teamId, userId) VALUES (?1, ?2)")
        {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        // let transaction = conn.transaction().unwrap();

        for user_id in members {
            let _ = match stmt.execute(params![team_id, user_id]) {
                Ok(r) => Ok(true),
                Err(err) => Err(DatastoreError::InternalError(
                    ("Faild to insert data").to_string(),
                )),
            };
        }
        // transaction.commit().unwrap();
        Ok(true)
    }

    pub fn remove_member(
        &self,
        conn: &Connection,
        team_id: i32,
        member_id: i32,
    ) -> Result<bool, DatastoreError> {
        let mut stmt = match conn.prepare("DELETE FROM TeamsUsers WHERE id = ?1") {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        stmt.execute(params![member_id]).unwrap();
        Ok(true)
    }

    pub fn get_user_teams(
        &self,
        conn: &Connection,
        user_id: i32,
    ) -> Result<Vec<TeamUserModel>, DatastoreError> {
        let mut stmt = match conn.prepare(
            "SELECT t.id, t.name, t.description FROM TeamsUsers tu
                    INNER Join Teams t on tu.teamId = t.id
                    where tu.userId=?1
        ",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let rows = match stmt.query_map(params![user_id], |row| {
            Ok(TeamUserModel {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
            })
        }) {
            Ok(teams) => teams,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let mut teams: Vec<TeamUserModel> = Vec::new();
        for team in rows {
            match team {
                Ok(t) => teams.push(t),
                Err(err) => warn!("Bad data"),
            }
        }
        Ok(teams)
    }

    pub fn update_configuration(
        &self,
        conn: &Connection,
        team_id: i32,
        apps: String,
    ) -> Result<bool, DatastoreError> {
        let mut stmt =
            match conn.prepare("UPDATE TeamConfiguration set apps = ?1 where teamId = ?2") {
                Ok(stmt) => stmt,
                Err(err) => {
                    return Err(DatastoreError::InternalError(format!(
                        "Failed to prepare get_value SQL statement: {err}"
                    )))
                }
            };
        match stmt.execute(params![apps, team_id]) {
            Ok(r) => Ok(true),
            Err(err) => Err(DatastoreError::InternalError(
                ("Faild to insert data").to_string(),
            )),
        }
    }

    pub fn add_configuration(
        &self,
        conn: &Connection,
        team_id: i32,
        apps: String,
    ) -> Result<bool, DatastoreError> {
        let mut stmt =
            match conn.prepare("INSERT INTO TeamConfiguration (teamId, apps) VALUES (?1, ?2)") {
                Ok(stmt) => stmt,
                Err(err) => {
                    return Err(DatastoreError::InternalError(format!(
                        "Failed to prepare get_value SQL statement: {err}"
                    )))
                }
            };
        match stmt.execute(params![team_id, apps]) {
            Ok(r) => Ok(true),
            Err(err) => Err(DatastoreError::InternalError(
                ("Faild to insert data").to_string(),
            )),
        }
    }

    pub fn get_configuration(
        &self,
        conn: &Connection,
        team_id: i32,
    ) -> Result<TeamConfiguration, DatastoreError> {
        let mut stmt = match conn.prepare(
            "SELECT tc.id, tc.teamId, tc.apps FROM TeamConfiguration tc
                    where tc.teamId=?1
        ",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        let config = match stmt.query_row(params![team_id], |row| {
            let id = row.get(0)?;
            let team_id = row.get(1)?;
            let apps_string = row.get::<usize, String>(2)?;
            let apps: Vec<String>;
            if apps_string.len() > 0 {
                apps = apps_string.split(',').map(|s| s.to_string()).collect();
            } else {
                apps = Vec::new();
            }
            return Ok(TeamConfiguration {
                id: id,
                team_id: team_id,
                apps: apps,
            });
        }) {
            Ok(config) => config,
            Err(err) => {
                return Err(DatastoreError::InternalError(format!(
                    "Failed to prepare get_value SQL statement: {err}"
                )))
            }
        };
        Ok(config)
    }
}
