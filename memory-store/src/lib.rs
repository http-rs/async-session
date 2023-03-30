use async_session::{async_trait, Session, SessionStore};
use dashmap::{mapref::entry::Entry::Occupied, DashMap};
use std::sync::Arc;

/// # In-memory session store
///
/// Because there is no external persistance, this session store is
/// ephemeral and will be cleared on server restart.
///
/// ## ***READ THIS BEFORE USING IN A PRODUCTION DEPLOYMENT***
///
/// Storing sessions only in memory brings the following problems:
///
/// 1. All sessions must fit in available memory.
/// 2. Sessions stored in memory are cleared only if a client calls [MemoryStore::destroy_session] or [MemoryStore::clear_store].
///    If sessions are not cleaned up properly it might result in OOM.
/// 3. All sessions will be lost on shutdown.
/// 4. If the service is clustered particular session will be stored only on a single instance.
///
/// See the crate readme for preferable session stores.
///
#[derive(Default, Debug, Clone)]
pub struct MemoryStore(Arc<DashMap<String, Session>>);

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
/// All errors that can occur in [`MemoryStore`]
pub enum MemoryStoreError {
    /// A base64 error
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),

    /// A json error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[async_trait]
impl SessionStore for MemoryStore {
    type Error = MemoryStoreError;

    async fn load_session(&self, cookie_value: &str) -> Result<Option<Session>, Self::Error> {
        let id = Session::id_from_cookie_value(cookie_value)?;
        log::trace!("loading session by id `{}`", id);
        let Occupied(entry) = self.0.entry(id) else {
            return Ok(None);
        };

        if entry.get().is_expired() {
            entry.remove();
            Ok(None)
        } else {
            Ok(Some(entry.get().clone()))
        }
    }

    async fn store_session(&self, session: &mut Session) -> Result<Option<String>, Self::Error> {
        log::trace!("storing session by id `{}`", session.id());
        session.reset_data_changed();
        let cookie_value = session.take_cookie_value();
        self.0.insert(session.id().to_string(), session.clone());
        Ok(cookie_value)
    }

    async fn destroy_session(&self, session: &mut Session) -> Result<(), Self::Error> {
        log::trace!("destroying session by id `{}`", session.id());
        self.0.remove(session.id());
        Ok(())
    }

    async fn clear_store(&self) -> Result<(), Self::Error> {
        log::trace!("clearing memory store");
        self.0.clear();
        Ok(())
    }
}

impl MemoryStore {
    /// Create a new instance of MemoryStore
    pub fn new() -> Self {
        Self::default()
    }

    /// Performs session cleanup. This should be run on an
    /// intermittent basis if this store is run for long enough that
    /// memory accumulation is a concern
    pub fn cleanup(&self) {
        log::trace!("cleaning up memory store...");
        self.0.retain(|_, session| !session.is_expired());
    }

    /// returns the number of elements in the memory store
    /// # Example
    /// ```rust
    /// # use async_session::{Session, SessionStore};
    /// # use async_session_memory_store::MemoryStore;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { async_std::task::block_on(async {
    /// let mut store = MemoryStore::new();
    /// assert_eq!(store.count(), 0);
    /// store.store_session(&mut Session::new()).await?;
    /// assert_eq!(store.count(), 1);
    /// # Ok(()) }) }
    /// ```
    pub fn count(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use std::time::Duration;
    #[async_std::test]
    async fn creating_a_new_session_with_no_expiry() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        let mut session = Session::new();
        session.insert("key", "Hello")?;
        let cloned = session.clone();
        let cookie_value = store.store_session(&mut session).await?.unwrap();
        let loaded_session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(cloned.id(), loaded_session.id());
        assert_eq!("Hello", &loaded_session.get::<String>("key").unwrap());
        assert!(!loaded_session.is_expired());
        assert!(loaded_session.validate().is_some());
        Ok(())
    }

    #[async_std::test]
    async fn updating_a_session() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        let mut session = Session::new();

        session.insert("key", "value")?;
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let mut session = store.load_session(&cookie_value).await?.unwrap();
        session.insert("key", "other value")?;

        assert_eq!(store.store_session(&mut session).await?, None);
        let session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(&mut session.get::<String>("key").unwrap(), "other value");

        Ok(())
    }

    #[async_std::test]
    async fn updating_a_session_extending_expiry() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(1));
        let original_expires = *session.expiry().unwrap();
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let mut session = store.load_session(&cookie_value).await?.unwrap();

        assert_eq!(session.expiry().unwrap(), &original_expires);
        session.expire_in(Duration::from_secs(3));
        let new_expires = *session.expiry().unwrap();
        assert_eq!(None, store.store_session(&mut session).await?);

        let session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(session.expiry().unwrap(), &new_expires);

        task::sleep(Duration::from_secs(3)).await;
        assert_eq!(None, store.load_session(&cookie_value).await?);

        Ok(())
    }

    #[async_std::test]
    async fn creating_a_new_session_with_expiry() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(3));
        session.insert("key", "value")?;
        let cloned = session.clone();

        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let loaded_session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(cloned.id(), loaded_session.id());
        assert_eq!("value", &*loaded_session.get::<String>("key").unwrap());

        assert!(!loaded_session.is_expired());

        task::sleep(Duration::from_secs(3)).await;
        assert_eq!(None, store.load_session(&cookie_value).await?);

        Ok(())
    }

    #[async_std::test]
    async fn destroying_a_single_session() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        for _ in 0..3i8 {
            store.store_session(&mut Session::new()).await?;
        }

        let cookie = store.store_session(&mut Session::new()).await?.unwrap();
        assert_eq!(4, store.count());
        let mut session = store.load_session(&cookie).await?.unwrap();
        store.destroy_session(&mut session).await?;
        assert_eq!(None, store.load_session(&cookie).await?);
        assert_eq!(3, store.count());

        // attempting to destroy the session again is not an error
        assert!(store.destroy_session(&mut session).await.is_ok());
        Ok(())
    }

    #[async_std::test]
    async fn clearing_the_whole_store() -> Result<(), MemoryStoreError> {
        let store = MemoryStore::new();
        for _ in 0..3i8 {
            store.store_session(&mut Session::new()).await?;
        }

        assert_eq!(3, store.count());
        store.clear_store().await.unwrap();
        assert_eq!(0, store.count());

        Ok(())
    }
}
