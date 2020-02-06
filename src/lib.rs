//! Async HTTP sessions.
//!
//! This crate provides a generic interface between cookies and storage
//! backends to create a concept of sessions. It provides an interface that
//! can be used to encode and store sessions, and decode and load sessions
//! generating cookies in the process.
//!
//! # Security
//!
//! This module has not been vetted for security purposes, and in particular
//! the in-memory storage backend is wildly insecure. Please thoroughly
//! validate whether this crate is a match for your intended use case before
//! relying on it in any sensitive context.
//!
//! # Examples
//!
//! ```
//! use async_session::mem::MemoryStore;
//! use async_session::{Session, SessionStore};
//! use cookie::CookieJar;
//!
//! # fn main() -> std::io::Result<()> {
//! # async_std::task::block_on(async {
//! #
//! // Init a new session store we can persist sessions to.
//! let mut store = MemoryStore::new();
//!
//! // Create a new session.
//! let sess = store.create_session();
//!
//! // Persist the session to our backend, and store a cookie
//! // to later access the session.
//! let mut jar = CookieJar::new();
//! let sess = store.store_session(sess, &mut jar).await?;
//!
//! // Retrieve the session using the cookie.
//! let sess = store.load_session(&jar).await?;
//! println!("session: {:?}", sess);
//! #
//! # Ok(()) }) }
//! ```

#![forbid(unsafe_code, future_incompatible, rust_2018_idioms)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, missing_doc_code_examples, unreachable_pub)]

use async_trait::async_trait;
use std::collections::HashMap;

/// An async session backend.
#[async_trait]
pub trait SessionStore: Send + Sync + 'static + Clone {
    /// The type of error that can occur when storing and loading errors.
    type Error;

    /// Get a session from the storage backend.
    ///
    /// The input should usually be the content of a cookie. This will then be
    /// parsed by the session middleware into a valid session.
    async fn load_session(&self, jar: &cookie::CookieJar) -> Result<Session, Self::Error>;

    /// Store a session on the storage backend.
    ///
    /// This method should return a stringified representation of the session so
    /// that it can be sent back to the client through a cookie.
    async fn store_session(
        &mut self,
        session: Session,
        jar: &mut cookie::CookieJar,
    ) -> Result<(), Self::Error>;
}

/// The main session type.
#[derive(Clone, Debug)]
pub struct Session {
    inner: HashMap<String, String>,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insert a new value into the Session.
    pub fn insert(&mut self, k: String, v: String) -> Option<String> {
        self.inner.insert(k, v)
    }

    /// Get a value from the session.
    pub fn get(&self, k: &str) -> Option<&String> {
        self.inner.get(k)
    }
}

/// In-memory session store.
pub mod mem {
    use async_std::io::{Error, ErrorKind};
    use async_std::sync::{Arc, RwLock};
    use cookie::Cookie;
    use std::collections::HashMap;

    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::{Session, SessionStore};

    /// An in-memory session store.
    ///
    /// # Security
    ///
    /// This store *does not* generate secure sessions, and should under no
    /// circumstance be used in production. It's meant only to quickly create
    /// sessions.
    #[derive(Debug)]
    pub struct MemoryStore {
        inner: Arc<RwLock<HashMap<String, Session>>>,
    }

    impl MemoryStore {
        /// Create a new instance of MemoryStore.
        pub fn new() -> Self {
            Self {
                inner: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        /// Generates a new session by generating a new uuid.
        ///
        /// This is *not* a secure way of generating sessions, and is intended for debug purposes only.
        pub fn create_session(&self) -> Session {
            let mut sess = Session::new();
            sess.insert("id".to_string(), uuid::Uuid::new_v4().to_string());
            sess
        }
    }

    impl Clone for MemoryStore {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    #[async_trait]
    impl SessionStore for MemoryStore {
        /// The type of error that can occur when storing and loading errors.
        type Error = std::io::Error;

        /// Get a session from the storage backend.
        async fn load_session(&self, jar: &cookie::CookieJar) -> Result<Session, Self::Error> {
            let id = match jar.get("session") {
                Some(cookie) => Uuid::parse_str(cookie.value()),
                None => return Err(Error::new(ErrorKind::Other, "No session cookie found")),
            };

            let id = id
                .map_err(|_| Error::new(ErrorKind::Other, "Cookie content was not a valid uuid"))?
                .to_string();

            let inner = self.inner.read().await;
            let sess = inner.get(&id).ok_or(Error::from(ErrorKind::Other))?;
            Ok(sess.clone())
        }

        /// Store a session on the storage backend.
        ///
        /// The data inside the session will be url-encoded so it can be stored
        /// inside a cookie.
        async fn store_session(
            &mut self,
            sess: Session,
            jar: &mut cookie::CookieJar,
        ) -> Result<(), Self::Error> {
            let mut inner = self.inner.write().await;
            let id = sess.get("id").unwrap().to_string();
            inner.insert(id.clone(), sess);
            jar.add(Cookie::new("session", id));
            Ok(())
        }
    }
}
