use async_trait::async_trait;
use redis::aio::MultiplexedConnection;
use serde::{de::DeserializeOwned,  Serialize};
use std::time::Duration;

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
    client: redis::Client,
    manager: MultiplexedConnection,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let manager = client.get_multiplexed_async_connection().await?;

        Ok(Self { client, manager })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.manager.clone();
        let result: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
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
        let mut conn = self.manager.clone();
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
            .query_async(&mut conn)
            .await
    }

    async fn flush(&self) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
    }
}
