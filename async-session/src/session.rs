use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, convert::TryFrom};
use time::OffsetDateTime as DateTime;

/// # The main session type.
///
/// ## Cloning and Serialization
///
/// The `cookie_value` field is not serialized, and it can only be
/// read through `into_cookie_value`. The intent of this field is that
/// it is set either by initialization or by a session store, and read
/// exactly once in order to set the cookie value.
///
/// ## Change tracking session tracks whether any of its inner data
/// was changed since it was last serialized. Any session store that
/// does not undergo a serialization-deserialization cycle must call
/// [`Session::reset_data_changed`] in order to reset the change tracker on
/// an individual record.
///
/// ### Change tracking example
/// ```rust
/// # use async_session::Session;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
/// let mut session = Session::new();
/// assert!(!session.data_changed());
///
/// session.insert("key", 1)?;
/// assert!(session.data_changed());
///
/// session.reset_data_changed();
/// assert_eq!(session.get::<usize>("key").unwrap(), 1);
/// assert!(!session.data_changed());
///
/// session.insert("key", 2)?;
/// assert!(session.data_changed());
/// assert_eq!(session.get::<usize>("key").unwrap(), 2);
///
/// session.insert("key", 1)?;
/// assert!(session.data_changed(), "reverting the data still counts as a change");
///
/// session.reset_data_changed();
/// assert!(!session.data_changed());
/// session.remove("nonexistent key");
/// assert!(!session.data_changed());
/// session.remove("key");
/// assert!(session.data_changed());
/// # Ok(()) }) }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    id: String,
    expiry: Option<DateTime>,
    data: HashMap<String, Value>,

    #[serde(skip)]
    cookie_value: Option<String>,
    #[serde(skip)]
    data_changed: bool,
    #[serde(skip)]
    destroy: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// generates a random cookie value
fn generate_cookie(len: usize) -> String {
    let mut key = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut key);
    BASE64.encode(key)
}

impl Session {
    /// Create a new session. Generates a random id and matching
    /// cookie value. Does not set an expiry by default
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let session = Session::new();
    /// assert_eq!(None, session.expiry());
    /// assert!(session.into_cookie_value().is_some());
    /// # Ok(()) }) }
    pub fn new() -> Self {
        let cookie_value = generate_cookie(64);
        let id = Session::id_from_cookie_value(&cookie_value).unwrap();

        Self {
            data_changed: false,
            expiry: None,
            data: HashMap::default(),
            cookie_value: Some(cookie_value),
            id,
            destroy: false,
        }
    }

    /// Create a session from id, data, and expiry. This is intended
    /// to be used by session store implementers to rehydrate sessions
    /// from persistence.
    pub fn from_parts(id: String, data: HashMap<String, Value>, expiry: Option<DateTime>) -> Self {
        Self {
            data,
            expiry,
            id,
            data_changed: false,
            destroy: false,
            cookie_value: None,
        }
    }

    /// Borrow the data hashmap. This is intended to be used by
    /// session store implementers.
    pub fn data(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// applies a cryptographic hash function on a cookie value
    /// returned by [`Session::into_cookie_value`] to obtain the
    /// session id for that cookie. Returns an error if the cookie
    /// format is not recognized
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let session = Session::new();
    /// let id = session.id().to_string();
    /// let cookie_value = session.into_cookie_value().unwrap();
    /// assert_eq!(id, Session::id_from_cookie_value(&cookie_value)?);
    /// # Ok(()) }) }
    /// ```
    pub fn id_from_cookie_value(string: &str) -> Result<String, base64::DecodeError> {
        let decoded = BASE64.decode(string)?;
        let hash = blake3::hash(&decoded);
        Ok(BASE64.encode(hash.as_bytes()))
    }

    /// mark this session for destruction. the actual session record
    /// is not destroyed until the end of this response cycle.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert!(!session.is_destroyed());
    /// session.destroy();
    /// assert!(session.is_destroyed());
    /// # Ok(()) }) }
    pub fn destroy(&mut self) {
        self.destroy = true;
    }

    /// returns true if this session is marked for destruction
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert!(!session.is_destroyed());
    /// session.destroy();
    /// assert!(session.is_destroyed());
    /// # Ok(()) }) }

    pub fn is_destroyed(&self) -> bool {
        self.destroy
    }

    /// Gets the session id
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let session = Session::new();
    /// let id = session.id().to_owned();
    /// let cookie_value = session.into_cookie_value().unwrap();
    /// assert_eq!(id, Session::id_from_cookie_value(&cookie_value)?);
    /// # Ok(()) }) }
    pub fn id(&self) -> &str {
        &self.id
    }

    /// inserts a serializable value into the session hashmap. returns
    /// an error if the serialization was unsuccessful.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use serde::{Serialize, Deserialize};
    /// # use async_session::Session;
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     name: String,
    ///     legs: u8
    /// }
    /// let mut session = Session::new();
    /// session.insert("user", User { name: "chashu".into(), legs: 4 }).expect("serializable");
    /// assert_eq!(r#"{"legs":4,"name":"chashu"}"#, session.get_value("user").unwrap().to_string());
    /// ```
    pub fn insert(&mut self, key: &str, value: impl Serialize) -> Result<(), serde_json::Error> {
        self.insert_value(key, serde_json::to_value(&value)?);
        Ok(())
    }

    /// inserts a string into the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// session.insert_value("ten", serde_json::json!(10));
    /// let ten: usize = session.get("ten").unwrap();
    /// assert_eq!(ten, 10);
    /// ```
    pub fn insert_value(&mut self, key: &str, value: Value) {
        if self.data.get(key) != Some(&value) {
            self.data.insert(key.to_string(), value);
            self.data_changed = true;
        }
    }

    /// deserializes a type T out of the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// session.insert("key", vec![1, 2, 3]);
    /// let numbers: Vec<usize> = session.get("key").unwrap();
    /// assert_eq!(vec![1, 2, 3], numbers);
    /// ```
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_value(key)
            .map(serde_json::from_value)
            .transpose()
            .ok()
            .flatten()
    }

    /// returns the [`serde_json::Value`] contained in the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// session.insert("key", vec![1, 2, 3]);
    /// assert_eq!("[1,2,3]", session.get_value("key").unwrap().to_string());
    /// ```
    pub fn get_value(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }

    /// removes an entry from the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// session.insert("key", "value");
    /// session.remove("key");
    /// assert!(session.get_value("key").is_none());
    /// assert_eq!(session.len(), 0);
    /// ```
    pub fn remove(&mut self, key: &str) {
        if self.data.remove(key).is_some() {
            self.data_changed = true;
        }
    }

    /// Takes an entry from the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// session.insert("key", "value");
    /// let took = session.take_value("key").unwrap();
    /// assert_eq!(took.to_string(), "\"value\"");
    /// assert!(session.get_value("key").is_none());
    /// assert_eq!(session.len(), 0);
    /// ```
    pub fn take_value(&mut self, key: &str) -> Option<Value> {
        let took = self.data.remove(key);
        if took.is_some() {
            self.data_changed = true;
        }
        took
    }

    /// returns the number of elements in the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// assert_eq!(session.len(), 0);
    /// session.insert("key", 0);
    /// assert_eq!(session.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// returns a boolean indicating whether there are zero elements in the session hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// let mut session = Session::new();
    /// assert!(session.is_empty());
    /// session.insert("key", 0);
    /// assert!(!session.is_empty());
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Generates a new id and cookie for this session
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// let old_id = session.id().to_string();
    /// session.regenerate();
    /// assert!(session.id() != &old_id);
    /// let new_id = session.id().to_string();
    /// let cookie_value = session.into_cookie_value().unwrap();
    /// assert_eq!(new_id, Session::id_from_cookie_value(&cookie_value)?);
    /// # Ok(()) }) }
    /// ```
    pub fn regenerate(&mut self) {
        let cookie_value = generate_cookie(64);
        self.id = Session::id_from_cookie_value(&cookie_value).unwrap();
        self.cookie_value = Some(cookie_value);
    }

    /// sets the cookie value that this session will use to serialize
    /// itself. this should only be called by cookie stores. any other
    /// uses of this method will result in the cookie not getting
    /// correctly deserialized on subsequent requests.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// session.set_cookie_value("hello".to_owned());
    /// let cookie_value = session.into_cookie_value().unwrap();
    /// assert_eq!(cookie_value, "hello".to_owned());
    /// # Ok(()) }) }
    /// ```
    pub fn set_cookie_value(&mut self, cookie_value: String) {
        self.cookie_value = Some(cookie_value)
    }

    /// returns the expiry timestamp of this session, if there is one
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert_eq!(None, session.expiry());
    /// session.expire_in(std::time::Duration::from_secs(1));
    /// assert!(session.expiry().is_some());
    /// # Ok(()) }) }
    /// ```
    pub fn expiry(&self) -> Option<&DateTime> {
        self.expiry.as_ref()
    }

    /// assigns an expiry timestamp to this session
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert_eq!(None, session.expiry());
    /// session.set_expiry(time::OffsetDateTime::now_utc());
    /// assert!(session.expiry().is_some());
    /// # Ok(()) }) }
    /// ```
    pub fn set_expiry(&mut self, expiry: DateTime) {
        self.expiry = Some(expiry);
    }

    /// assigns the expiry timestamp to a duration from the current time.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert_eq!(None, session.expiry());
    /// session.expire_in(std::time::Duration::from_secs(1));
    /// assert!(session.expiry().is_some());
    /// # Ok(()) }) }
    /// ```
    pub fn expire_in(&mut self, ttl: std::time::Duration) {
        self.expiry = Some(DateTime::now_utc() + ttl);
    }

    /// predicate function to determine if this session is
    /// expired. returns false if there is no expiry set, or if it is
    /// in the past.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # use std::time::Duration;
    /// # use async_std::task;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert_eq!(None, session.expiry());
    /// assert!(!session.is_expired());
    /// session.expire_in(Duration::from_secs(1));
    /// assert!(!session.is_expired());
    /// task::sleep(Duration::from_secs(2)).await;
    /// assert!(session.is_expired());
    /// # Ok(()) }) }
    /// ```
    pub fn is_expired(&self) -> bool {
        match self.expiry {
            Some(expiry) => expiry < DateTime::now_utc(),
            None => false,
        }
    }

    /// Ensures that this session is not expired. Returns None if it is expired
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # use std::time::Duration;
    /// # use async_std::task;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let session = Session::new();
    /// let mut session = session.validate().unwrap();
    /// session.expire_in(Duration::from_secs(1));
    /// let session = session.validate().unwrap();
    /// task::sleep(Duration::from_secs(2)).await;
    /// assert_eq!(None, session.validate());
    /// # Ok(()) }) }
    /// ```
    pub fn validate(self) -> Option<Self> {
        if self.is_expired() {
            None
        } else {
            Some(self)
        }
    }

    /// Checks if the data has been modified. This is based on the
    /// implementation of [`PartialEq`] for the inner data type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert!(!session.data_changed(), "new session is not changed");
    /// session.insert("key", 1);
    /// assert!(session.data_changed());
    ///
    /// session.reset_data_changed();
    /// assert!(!session.data_changed());
    /// session.remove("key");
    /// assert!(session.data_changed());
    /// # Ok(()) }) }
    /// ```
    pub fn data_changed(&self) -> bool {
        self.data_changed
    }

    /// Resets `data_changed` dirty tracking. This is unnecessary for
    /// any session store that serializes the data to a string on
    /// storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// assert!(!session.data_changed(), "new session is not changed");
    /// session.insert("key", 1);
    /// assert!(session.data_changed());
    ///
    /// session.reset_data_changed();
    /// assert!(!session.data_changed());
    /// session.remove("key");
    /// assert!(session.data_changed());
    /// # Ok(()) }) }
    /// ```
    pub fn reset_data_changed(&mut self) {
        self.data_changed = false;
    }

    /// Ensures that this session is not expired. Returns None if it is expired
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # use std::time::Duration;
    /// # use async_std::task;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// session.expire_in(Duration::from_secs(123));
    /// let expires_in = session.expires_in().unwrap();
    /// assert!(123 - expires_in.as_secs() < 2);
    /// # Ok(()) }) }
    /// ```
    /// Duration from now to the expiry time of this session
    pub fn expires_in(&self) -> Option<std::time::Duration> {
        let dur = self.expiry? - DateTime::now_utc();
        if dur.is_negative() {
            None
        } else {
            std::time::Duration::try_from(dur).ok()
        }
    }

    /// takes the cookie value and consume this session.
    /// this is generally only performed by the session store
    ///
    /// # Example
    ///
    /// ```rust
    /// # use async_session::Session;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut session = Session::new();
    /// session.set_cookie_value("hello".to_owned());
    /// let cookie_value = session.into_cookie_value().unwrap();
    /// assert_eq!(cookie_value, "hello".to_owned());
    /// # Ok(()) }) }
    /// ```
    pub fn into_cookie_value(mut self) -> Option<String> {
        self.take_cookie_value()
    }

    /// take the cookie value. this is generally only performed by a
    /// session store.
    pub fn take_cookie_value(&mut self) -> Option<String> {
        self.cookie_value.take()
    }
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
}
