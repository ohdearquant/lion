use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A trait for entities that can be uniquely identified
pub trait Identifiable {
    fn id(&self) -> Uuid;
}

/// A trait for entities that have timestamps
pub trait Timestamped {
    fn created_at(&self) -> DateTime<Utc>;
    fn updated_at(&self) -> Option<DateTime<Utc>>;
}

/// A trait for entities that can be serialized/deserialized
pub trait DataFormat: Serialize + for<'de> Deserialize<'de> {}

/// A trait for entities that can be stored and retrieved
#[async_trait]
pub trait Storable: Identifiable + DataFormat {
    type Error;
    
    async fn save(&self) -> Result<(), Self::Error>;
    async fn load(id: Uuid) -> Result<Self, Self::Error> where Self: Sized;
    async fn delete(id: Uuid) -> Result<(), Self::Error>;
}

/// A trait for entities that can be validated
pub trait Validatable {
    type Error;
    
    fn validate(&self) -> Result<(), Self::Error>;
}

/// A trait for entities that can be cloned with modifications
pub trait Modifiable: Clone {
    fn with_id(self, id: Uuid) -> Self;
    fn with_timestamp(self, timestamp: DateTime<Utc>) -> Self;
}

/// A trait for entities that can be converted to/from JSON
pub trait JsonFormat {
    fn to_json(&self) -> serde_json::Result<serde_json::Value>;
    fn from_json(value: serde_json::Value) -> serde_json::Result<Self> where Self: Sized;
}

/// A trait for entities that can be versioned
pub trait Versionable {
    fn version(&self) -> String;
    fn is_compatible_with(&self, other_version: &str) -> bool;
}

/// A trait for entities that can be described
pub trait Describable {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

/// A trait for entities that can be enabled/disabled
pub trait Toggleable {
    fn is_enabled(&self) -> bool;
    fn enable(&mut self);
    fn disable(&mut self);
}

/// A trait for entities that can handle events
#[async_trait]
pub trait EventHandler {
    type Event;
    type Response;
    type Error;

    async fn handle(&self, event: Self::Event) -> Result<Self::Response, Self::Error>;
}

/// A trait for entities that can be initialized
#[async_trait]
pub trait Initializable {
    type Config;
    type Error;

    async fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
}