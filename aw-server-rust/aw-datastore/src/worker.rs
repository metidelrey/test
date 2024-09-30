use std::collections::HashMap;
use std::collections::LinkedList;
use std::fmt;
use std::thread;

use aw_models::Member;
use aw_models::PublicUser;
use aw_models::TeamConfiguration;
use aw_models::TeamUserModel;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;

use rusqlite::Connection;
use rusqlite::DropBehavior;
use rusqlite::Transaction;
use rusqlite::TransactionBehavior;

use aw_models::Bucket;
use aw_models::Event;
use aw_models::Team;
use aw_models::TeamRequestModel;
use aw_models::User;

use crate::DatastoreError;
use crate::DatastoreInstance;
use crate::DatastoreMethod;

use mpsc_requests::ResponseReceiver;

type RequestSender = mpsc_requests::RequestSender<Command, Result<Response, DatastoreError>>;
type RequestReceiver = mpsc_requests::RequestReceiver<Command, Result<Response, DatastoreError>>;

#[derive(Clone)]
pub struct Datastore {
    requester: RequestSender,
}

impl fmt::Debug for Datastore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Datastore()")
    }
}

/*
 * TODO:
 * - Allow read requests to go straight through a read-only db connection instead of requesting the
 * worker thread for better performance?
 * TODO: Add an separate "Import" request which does an import with an transaction
 */

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Response {
    Empty(),
    User(User),
    PublicUser(PublicUser),
    Users(Vec<PublicUser>),
    Members(Vec<Member>),
    Teams(Vec<Team>),
    UserTeams(Vec<TeamUserModel>),
    Team(Team),
    TeamConfiguration(TeamConfiguration),
    Bucket(Bucket),
    BucketMap(HashMap<String, Bucket>),
    Event(Event),
    EventList(Vec<Event>),
    Count(i64),
    KeyValue(String),
    KeyValues(HashMap<String, String>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Command {
    CreateBucket(Bucket),
    DeleteBucket(i64),
    GetBucket(i64),
    GetBuckets(i32),
    InsertEvents(i64, Vec<Event>),
    Heartbeat(i64, Event, f64),
    GetEvent(i64, i64),
    GetEvents(
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<u64>,
    ),
    GetUserEvents(
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<u64>,
        Option<i32>,
    ),
    GetEventCount(i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>),
    DeleteEventsById(i64, Vec<i64>),
    ForceCommit(),
    GetKeyValues(String),
    GetKeyValue(String),
    SetKeyValue(String, String),
    DeleteKeyValue(String),
    Close(),
    GetUser(i32),
    GetUserByEmail(String),
    AddUser(User),
    GetTeams(i32),
    AddTeam(TeamRequestModel, i32),
    GetTeamMembersCount(i32), // EditTeam(Team),
    GetTeam(i32),
    GetAllUsers(),
    GetMembersOfTeam(i32),
    AddMembers(i32, Vec<i32>),
    RemoveMember(i32, i32),
    GetUserTeams(i32),
    AddTeamConfiguration(i32, String),
    UpdateTeamConfiguration(i32, String),
    GetTeamConfiguration(i32),
}

fn _unwrap_response(
    receiver: ResponseReceiver<Result<Response, DatastoreError>>,
) -> Result<(), DatastoreError> {
    match receiver.collect().unwrap() {
        Ok(r) => match r {
            Response::Empty() => Ok(()),
            _ => panic!("Invalid response"),
        },
        Err(e) => Err(e),
    }
}

struct DatastoreWorker {
    responder: RequestReceiver,
    legacy_import: bool,
    quit: bool,
    uncommitted_events: usize,
    commit: bool,
    last_heartbeat: HashMap<String, Option<Event>>,
}

impl DatastoreWorker {
    pub fn new(
        responder: mpsc_requests::RequestReceiver<Command, Result<Response, DatastoreError>>,
        legacy_import: bool,
    ) -> Self {
        DatastoreWorker {
            responder,
            legacy_import,
            quit: false,
            uncommitted_events: 0,
            commit: false,
            last_heartbeat: HashMap::new(),
        }
    }

    fn work_loop(&mut self, method: DatastoreMethod) {
        // Open SQLite connection
        let mut conn = match &method {
            DatastoreMethod::Memory() => {
                Connection::open_in_memory().expect("Failed to create in-memory datastore")
            }
            DatastoreMethod::File(path) => {
                Connection::open(path).expect("Failed to create datastore")
            }
        };
        let mut ds = DatastoreInstance::new(&conn, true).unwrap();

        // Ensure legacy import
        if self.legacy_import {
            let transaction = match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                Ok(transaction) => transaction,
                Err(err) => {
                    panic!("Unable to start immediate transaction on SQLite database! {err}")
                }
            };
            match ds.ensure_legacy_import(&transaction) {
                Ok(_) => (),
                Err(err) => error!("Failed to do legacy import: {:?}", err),
            }
            match transaction.commit() {
                Ok(_) => (),
                Err(err) => panic!("Failed to commit datastore transaction! {err}"),
            }
        }

        // Start handling and respond to requests
        loop {
            let last_commit_time: DateTime<Utc> = Utc::now();
            let mut tx: Transaction =
                match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                    Ok(tx) => tx,
                    Err(err) => {
                        error!("Unable to start transaction! {:?}", err);
                        // Wait 1s before retrying
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        continue;
                    }
                };
            tx.set_drop_behavior(DropBehavior::Commit);

            self.uncommitted_events = 0;
            self.commit = false;
            loop {
                let (request, response_sender) = match self.responder.poll() {
                    Ok((req, res_sender)) => (req, res_sender),
                    Err(err) => {
                        // All references to responder is gone, quit
                        error!("DB worker quitting, error: {err:?}");
                        self.quit = true;
                        break;
                    }
                };
                let response = self.handle_request(request, &mut ds, &tx);
                response_sender.respond(response);

                let now: DateTime<Utc> = Utc::now();
                let commit_interval_passed: bool = (now - last_commit_time) > Duration::seconds(15);
                if self.commit
                    || commit_interval_passed
                    || self.uncommitted_events > 100
                    || self.quit
                {
                    break;
                };
            }
            debug!(
                "Committing DB! Force commit {}, {} uncommitted events",
                self.commit, self.uncommitted_events
            );
            match tx.commit() {
                Ok(_) => (),
                Err(err) => panic!("Failed to commit datastore transaction! {err}"),
            }
            if self.quit {
                break;
            };
        }
        info!("DB Worker thread finished");
    }

    fn handle_request(
        &mut self,
        request: Command,
        ds: &mut DatastoreInstance,
        tx: &Transaction,
    ) -> Result<Response, DatastoreError> {
        match request {
            Command::CreateBucket(bucket) => match ds.create_bucket(tx, bucket) {
                Ok(id) => {
                    self.commit = true;
                    Ok(Response::Count(id))
                }
                Err(e) => Err(e),
            },
            Command::DeleteBucket(bucket_id) => match ds.delete_bucket(tx, bucket_id) {
                Ok(_) => {
                    self.commit = true;
                    Ok(Response::Empty())
                }
                Err(e) => Err(e),
            },
            Command::GetBucket(bucket_id) => match ds.get_bucket(bucket_id) {
                Ok(b) => Ok(Response::Bucket(b)),
                Err(e) => Err(e),
            },
            Command::GetBuckets(user_id) => Ok(Response::BucketMap(ds.get_buckets(tx, user_id))),
            Command::InsertEvents(bucket_id, events) => {
                match ds.insert_events(tx, bucket_id, events) {
                    Ok(events) => {
                        self.uncommitted_events += events.len();
                        self.last_heartbeat.insert(bucket_id.to_string(), None); // invalidate last_heartbeat cache
                        Ok(Response::EventList(events))
                    }
                    Err(e) => Err(e),
                }
            }
            Command::Heartbeat(bucket_id, event, pulsetime) => {
                match ds.heartbeat(tx, bucket_id, event, pulsetime, &mut self.last_heartbeat) {
                    Ok(e) => {
                        self.uncommitted_events += 1;
                        Ok(Response::Event(e))
                    }
                    Err(e) => Err(e),
                }
            }
            Command::GetEvent(bucket_id, event_id) => match ds.get_event(tx, bucket_id, event_id) {
                Ok(el) => Ok(Response::Event(el)),
                Err(e) => Err(e),
            },
            Command::GetEvents(bucket_id, starttime_opt, endtime_opt, limit_opt) => {
                match ds.get_events(tx, bucket_id, starttime_opt, endtime_opt, limit_opt) {
                    Ok(el) => Ok(Response::EventList(el)),
                    Err(e) => Err(e),
                }
            }
            Command::GetUserEvents(bucket_id, starttime_opt, endtime_opt, limit_opt, team_id) => {
                match ds.get_user_events(tx, bucket_id, starttime_opt, endtime_opt, limit_opt, team_id) {
                    Ok(el) => Ok(Response::EventList(el)),
                    Err(e) => Err(e),
                }
            }
            Command::GetEventCount(bucket_id, starttime_opt, endtime_opt) => {
                match ds.get_event_count(tx, bucket_id, starttime_opt, endtime_opt) {
                    Ok(n) => Ok(Response::Count(n)),
                    Err(e) => Err(e),
                }
            }
            Command::DeleteEventsById(bucket_id, event_ids) => {
                match ds.delete_events_by_id(tx, bucket_id, event_ids) {
                    Ok(()) => Ok(Response::Empty()),
                    Err(e) => Err(e),
                }
            }
            Command::ForceCommit() => {
                self.commit = true;
                Ok(Response::Empty())
            }
            Command::GetKeyValues(pattern) => match ds.get_key_values(tx, pattern.as_str()) {
                Ok(result) => Ok(Response::KeyValues(result)),
                Err(e) => Err(e),
            },
            Command::SetKeyValue(key, data) => match ds.insert_key_value(tx, &key, &data) {
                Ok(()) => Ok(Response::Empty()),
                Err(e) => Err(e),
            },
            Command::GetKeyValue(key) => match ds.get_key_value(tx, &key) {
                Ok(result) => Ok(Response::KeyValue(result)),
                Err(e) => Err(e),
            },
            Command::DeleteKeyValue(key) => match ds.delete_key_value(tx, &key) {
                Ok(()) => Ok(Response::Empty()),
                Err(e) => Err(e),
            },
            Command::GetUserByEmail(email) => match ds.get_user_by_email(tx, email) {
                Ok((user)) => Ok(Response::User((user))),
                Err(e) => Err(e),
            },
            Command::AddUser(user) => match ds.signup(tx, user) {
                Ok((result)) => Ok(Response::PublicUser(result)),
                Err(e) => Err(e),
            },

            Command::GetUser(userId) => match ds.get_user(tx, userId) {
                Ok((result)) => Ok(Response::PublicUser(result)),
                Err(e) => Err(e),
            },

            Command::GetTeams(ownerId) => match ds.get_teams(tx, ownerId) {
                Ok(teams) => Ok((Response::Teams(teams))),
                Err(e) => Err(e),
            },

            Command::AddTeam(team, ownerId) => match ds.add_team(tx, team, ownerId) {
                Ok(team) => Ok((Response::Empty())),
                Err(e) => Err(e),
            },

            Command::GetTeamMembersCount(team_id) => match ds.get_team_members_count(tx, team_id) {
                Ok(count) => Ok(Response::Count((count))),
                Err(e) => Err(e),
            },

            Command::GetMembersOfTeam(team_id) => match ds.get_team_members(tx, team_id) {
                Ok(members) => Ok(Response::Members((members))),
                Err(e) => Err(e),
            },
            Command::GetTeam(team_id) => match ds.get_team(tx, team_id) {
                Ok(team) => Ok(Response::Team(team)),
                Err(e) => Err(e),
            },

            Command::GetAllUsers() => match ds.get_all_users(tx) {
                Ok(users) => Ok(Response::Users(users)),
                Err(e) => Err(e),
            },

            Command::AddMembers(team_id, members) => match ds.add_members(tx, team_id, members) {
                Ok(team) => Ok(Response::Empty()),
                Err(e) => Err(e),
            },

            Command::RemoveMember(team_id, member_id) => {
                match ds.remove_member(tx, team_id, member_id) {
                    Ok(team) => Ok(Response::Empty()),
                    Err(e) => Err(e),
                }
            }

            Command::GetUserTeams(user_id) => match ds.get_user_teams(tx, user_id) {
                Ok(teams) => Ok(Response::UserTeams(teams)),
                Err(e) => Err(e),
            },

            Command::AddTeamConfiguration(team_id, apps) => {
                match ds.add_configuration(tx, team_id, apps) {
                    Ok(teams) => Ok(Response::Empty()),
                    Err(e) => Err(e),
                }
            }

            Command::UpdateTeamConfiguration(team_id, apps) => {
                match ds.update_configuration(tx, team_id, apps) {
                    Ok(teams) => Ok(Response::Empty()),
                    Err(e) => Err(e),
                }
            }

            Command::GetTeamConfiguration(team_id) => match ds.get_configuration(tx, team_id) {
                Ok(config) => Ok(Response::TeamConfiguration(config)),
                Err(e) => Err(e),
            },

            Command::Close() => {
                self.quit = true;
                Ok(Response::Empty())
            }
        }
    }
}

impl Datastore {
    pub fn new(dbpath: String, legacy_import: bool) -> Self {
        let method = DatastoreMethod::File(dbpath);
        Datastore::_new_internal(method, legacy_import)
    }

    pub fn new_in_memory(legacy_import: bool) -> Self {
        let method = DatastoreMethod::Memory();
        Datastore::_new_internal(method, legacy_import)
    }

    fn _new_internal(method: DatastoreMethod, legacy_import: bool) -> Self {
        let (requester, responder) =
            mpsc_requests::channel::<Command, Result<Response, DatastoreError>>();
        let _thread = thread::spawn(move || {
            let mut di = DatastoreWorker::new(responder, legacy_import);
            di.work_loop(method);
        });
        Datastore { requester }
    }

    pub fn create_bucket(&self, bucket: &Bucket) -> Result<i64, DatastoreError> {
        let cmd = Command::CreateBucket(bucket.clone());
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Count(id) => Ok(id),
                _ => Ok(-1),
            },
            Err(e) => Err(e),
        }
    }

    pub fn delete_bucket(&self, bucket_id: i64) -> Result<(), DatastoreError> {
        let cmd = Command::DeleteBucket(bucket_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_bucket(&self, bucket_id: i64) -> Result<Bucket, DatastoreError> {
        let cmd = Command::GetBucket(bucket_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Bucket(b) => Ok(b),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_buckets(&self, user_id: i32) -> Result<HashMap<String, Bucket>, DatastoreError> {
        let cmd = Command::GetBuckets(user_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::BucketMap(bm) => Ok(bm),
                e => Err(DatastoreError::InternalError(format!(
                    "Invalid response: {e:?}"
                ))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn insert_events(
        &self,
        bucket_id: i64,
        events: &[Event],
    ) -> Result<Vec<Event>, DatastoreError> {
        let cmd = Command::InsertEvents(bucket_id, events.to_vec());
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::EventList(events) => Ok(events),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn heartbeat(
        &self,
        bucket_id: i64,
        heartbeat: Event,
        pulsetime: f64,
    ) -> Result<Event, DatastoreError> {
        let cmd = Command::Heartbeat(bucket_id, heartbeat, pulsetime);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Event(e) => Ok(e),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_event(&self, bucket_id: i64, event_id: i64) -> Result<Event, DatastoreError> {
        let cmd = Command::GetEvent(bucket_id, event_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Event(el) => Ok(el),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_events(
        &self,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
        limit_opt: Option<u64>,
    ) -> Result<Vec<Event>, DatastoreError> {
        let cmd = Command::GetEvents(bucket_id, starttime_opt, endtime_opt, limit_opt);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::EventList(el) => Ok(el),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_user_events(
        &self,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
        limit_opt: Option<u64>,
        team_id: Option<i32>,
    ) -> Result<Vec<Event>, DatastoreError> {
        let cmd = Command::GetUserEvents(bucket_id, starttime_opt, endtime_opt, limit_opt, team_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::EventList(el) => Ok(el),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_event_count(
        &self,
        bucket_id: i64,
        starttime_opt: Option<DateTime<Utc>>,
        endtime_opt: Option<DateTime<Utc>>,
    ) -> Result<i64, DatastoreError> {
        let cmd = Command::GetEventCount(bucket_id, starttime_opt, endtime_opt);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Count(n) => Ok(n),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn delete_events_by_id(
        &self,
        bucket_id: i64,
        event_ids: Vec<i64>,
    ) -> Result<(), DatastoreError> {
        let cmd = Command::DeleteEventsById(bucket_id, event_ids);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn force_commit(&self) -> Result<(), DatastoreError> {
        let cmd = Command::ForceCommit();
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_key_values(&self, pattern: &str) -> Result<HashMap<String, String>, DatastoreError> {
        let cmd = Command::GetKeyValues(pattern.to_string());
        let receiver = self.requester.request(cmd).unwrap();

        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::KeyValues(value) => Ok(value),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_key_value(&self, key: &str) -> Result<String, DatastoreError> {
        let cmd = Command::GetKeyValue(key.to_string());
        let receiver = self.requester.request(cmd).unwrap();

        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::KeyValue(kv) => Ok(kv),
                _ => panic!("Invalid response"),
            },
            Err(e) => Err(e),
        }
    }

    pub fn set_key_value(&self, key: &str, data: &str) -> Result<(), DatastoreError> {
        let cmd = Command::SetKeyValue(key.to_string(), data.to_string());
        let receiver = self.requester.request(cmd).unwrap();

        _unwrap_response(receiver)
    }

    pub fn delete_key_value(&self, key: &str) -> Result<(), DatastoreError> {
        let cmd = Command::DeleteKeyValue(key.to_string());
        let receiver = self.requester.request(cmd).unwrap();

        _unwrap_response(receiver)
    }

    // Should block until worker has stopped
    pub fn close(&self) {
        info!("Sending close request to database");
        let receiver = self.requester.request(Command::Close()).unwrap();

        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => (),
                _ => panic!("Invalid response"),
            },
            Err(e) => panic!("Error closing database: {:?}", e),
        }
    }

    pub fn get_user_by_email(&self, email: String) -> Result<User, DatastoreError> {
        let cmd = Command::GetUserByEmail(email);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::User(user) => Ok(user),
                _ => Err(DatastoreError::NoUser()),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_user(&self, userId: i32) -> Result<PublicUser, DatastoreError> {
        let cmd = Command::GetUser(userId);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::PublicUser(user) => Ok(user),
                _ => Err(DatastoreError::NoUser()),
            },
            Err(e) => Err(e),
        }
    }

    pub fn add_user(&self, user: User) -> Result<PublicUser, DatastoreError> {
        let cmd = Command::AddUser(user);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::PublicUser(user) => Ok(user),
                _ => Err(DatastoreError::NoUser()),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_owner_teams(&self, ownerId: i32) -> Result<Vec<Team>, DatastoreError> {
        let cmd = Command::GetTeams(ownerId);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Teams(teams) => Ok(teams),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_user_teams(&self, user_id: i32) -> Result<Vec<TeamUserModel>, DatastoreError> {
        let cmd = Command::GetUserTeams(user_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::UserTeams(teams) => Ok(teams),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn add_team(&self, team: TeamRequestModel, ownerId: i32) -> Result<(), DatastoreError> {
        let cmd = Command::AddTeam(team, ownerId);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_team_members_count(&self, team_id: i32) -> Result<i64, DatastoreError> {
        let cmd = Command::GetTeamMembersCount(team_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Count(count) => Ok(count),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_team_members(&self, team_id: i32) -> Result<Vec<Member>, DatastoreError> {
        let cmd = Command::GetMembersOfTeam(team_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Members(members) => Ok(members),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_team(&self, team_id: i32) -> Result<Team, DatastoreError> {
        let cmd = Command::GetTeam(team_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Team(team) => Ok(team),
                _ => Err(DatastoreError::InternalError(("".to_string()))),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_all_users(&self) -> Result<Vec<PublicUser>, DatastoreError> {
        let cmd = Command::GetAllUsers();
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Users(users) => Ok(users),
                _ => Err(DatastoreError::NoUser()),
            },
            Err(e) => Err(e),
        }
    }

    pub fn add_members(&self, team_id: i32, members: Vec<i32>) -> Result<(), DatastoreError> {
        let cmd = Command::AddMembers(team_id, members);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => Err(DatastoreError::NoUser()),
            },
            Err(e) => Err(e),
        }
    }

    pub fn remove_member(&self, team_id: i32, member_id: i32) -> Result<(), DatastoreError> {
        let cmd = Command::RemoveMember(team_id, member_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => Err(DatastoreError::InternalError(
                    "Faild to remove member".to_string(),
                )),
            },
            Err(e) => Err(e),
        }
    }

    pub fn add_configuration(&self, team_id: i32, apps: String) -> Result<(), DatastoreError> {
        let cmd = Command::AddTeamConfiguration(team_id, apps);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => Err(DatastoreError::InternalError(
                    "Faild to add configuration".to_string(),
                )),
            },
            Err(e) => Err(e),
        }
    }

    pub fn update_configuration(&self, team_id: i32, apps: String) -> Result<(), DatastoreError> {
        let cmd = Command::UpdateTeamConfiguration(team_id, apps);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::Empty() => Ok(()),
                _ => Err(DatastoreError::InternalError(
                    "Faild to add configuration".to_string(),
                )),
            },
            Err(e) => Err(e),
        }
    }

    pub fn get_configuration(&self, team_id: i32) -> Result<TeamConfiguration, DatastoreError> {
        let cmd = Command::GetTeamConfiguration(team_id);
        let receiver = self.requester.request(cmd).unwrap();
        match receiver.collect().unwrap() {
            Ok(r) => match r {
                Response::TeamConfiguration(r) => Ok(r),
                _ => Err(DatastoreError::InternalError(
                    "Faild to get configuration".to_string(),
                )),
            },
            Err(e) => Err(e),
        }
    }
}
