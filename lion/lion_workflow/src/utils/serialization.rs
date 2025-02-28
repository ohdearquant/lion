use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// Serialize a HashMap with Id<T> keys as strings to fix "key must be a string" JSON serialization errors
pub fn serialize_id_map<K, V, S>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    K: Serialize + Hash + Eq + fmt::Display,
    V: Serialize,
    S: Serializer,
{
    let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        // Convert the Id<T> key to a string for JSON compatibility
        map_serializer.serialize_entry(&k.to_string(), v)?;
    }
    map_serializer.end()
}

/// Deserialize a HashMap with Id<T> keys from strings
pub fn deserialize_id_map<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    K: Deserialize<'de> + Hash + Eq + std::str::FromStr,
    K::Err: fmt::Display,
    V: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // First deserialize to a HashMap<String, V>
    let string_map: HashMap<String, V> = HashMap::deserialize(deserializer)?;

    // Then convert the keys back to Id<T>
    let mut id_map = HashMap::with_capacity(string_map.len());
    for (k_str, v) in string_map {
        let k = K::from_str(&k_str).map_err(serde::de::Error::custom)?;
        id_map.insert(k, v);
    }

    Ok(id_map)
}
