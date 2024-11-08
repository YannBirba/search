use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use bb8::RunError;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), redis::RedisError>;
    async fn flush(&self) -> Result<(), redis::RedisError>;
}

pub struct RedisCache {
    pool: Pool<RedisConnectionManager>,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let manager = RedisConnectionManager::new(redis_url)?;
        let pool = Pool::builder().build(manager).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.pool.get().await.map_err(|e| match e {
            RunError::User(e) => e,
            RunError::TimedOut => redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection timed out",
            )),
        }).ok()?;
        let result: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut *conn)
            .await
            .ok()?;

        result.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| match e {
            RunError::User(e) => e,
            RunError::TimedOut => redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection timed out",
            )),
        })?;
        let serialized = serde_json::to_string(value).map_err(|_| {
            redis::RedisError::from((
                redis::ErrorKind::InvalidClientConfig,
                "Serialization failed",
            ))
        })?;

        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl.as_secs())
            .arg(serialized)
            .query_async(&mut *conn)
            .await
    }

    async fn flush(&self) -> Result<(), redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| match e {
            RunError::User(e) => e,
            RunError::TimedOut => redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection timed out",
            )),
        })?;
        redis::cmd("FLUSHDB").query_async(&mut *conn).await
    }
}
