//! Configuration utilities.
//!
//! This module provides utilities for configuration management.
//! It defines a configuration value type that can represent various types of values.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A configuration value.
///
/// This enum represents different types of configuration values
/// that can be used in the system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// Null value.
    Null,

    /// Boolean value.
    Bool(bool),

    /// Integer value.
    Integer(i64),

    /// Floating-point value.
    Float(f64),

    /// String value.
    String(String),

    /// Array of values.
    Array(Vec<ConfigValue>),

    /// Map of values.
    Map(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    /// Check if this value is null.
    ///
    /// # Returns
    ///
    /// `true` if this value is null, `false` otherwise.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this value is a boolean.
    ///
    /// # Returns
    ///
    /// `true` if this value is a boolean, `false` otherwise.
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// Check if this value is an integer.
    ///
    /// # Returns
    ///
    /// `true` if this value is an integer, `false` otherwise.
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    /// Check if this value is a floating-point number.
    ///
    /// # Returns
    ///
    /// `true` if this value is a floating-point number, `false` otherwise.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Check if this value is a number (integer or float).
    ///
    /// # Returns
    ///
    /// `true` if this value is a number, `false` otherwise.
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Integer(_) | Self::Float(_))
    }

    /// Check if this value is a string.
    ///
    /// # Returns
    ///
    /// `true` if this value is a string, `false` otherwise.
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Check if this value is an array.
    ///
    /// # Returns
    ///
    /// `true` if this value is an array, `false` otherwise.
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Check if this value is a map.
    ///
    /// # Returns
    ///
    /// `true` if this value is a map, `false` otherwise.
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Get this value as a boolean.
    ///
    /// # Returns
    ///
    /// The boolean value, or `None` if this value is not a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get this value as an integer.
    ///
    /// # Returns
    ///
    /// The integer value, or `None` if this value is not an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Float(f)
                if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 =>
            {
                Some(*f as i64)
            }
            _ => None,
        }
    }

    /// Get this value as a floating-point number.
    ///
    /// # Returns
    ///
    /// The floating-point value, or `None` if this value is not a number.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get this value as a string.
    ///
    /// # Returns
    ///
    /// The string value, or `None` if this value is not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get this value as an array.
    ///
    /// # Returns
    ///
    /// The array value, or `None` if this value is not an array.
    pub fn as_array(&self) -> Option<&[ConfigValue]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get this value as a map.
    ///
    /// # Returns
    ///
    /// The map value, or `None` if this value is not a map.
    pub fn as_map(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get a mutable reference to this value as an array.
    ///
    /// # Returns
    ///
    /// A mutable reference to the array value, or `None` if this value is not an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<ConfigValue>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get a mutable reference to this value as a map.
    ///
    /// # Returns
    ///
    /// A mutable reference to the map value, or `None` if this value is not a map.
    pub fn as_map_mut(&mut self) -> Option<&mut HashMap<String, ConfigValue>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get a value from an array by index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to get.
    ///
    /// # Returns
    ///
    /// The value at the given index, or `None` if this value is not an array
    /// or the index is out of bounds.
    pub fn get_index(&self, index: usize) -> Option<&ConfigValue> {
        match self {
            Self::Array(a) => a.get(index),
            _ => None,
        }
    }

    /// Get a value from a map by key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the value to get.
    ///
    /// # Returns
    ///
    /// The value with the given key, or `None` if this value is not a map
    /// or the key does not exist.
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        match self {
            Self::Map(m) => m.get(key),
            _ => None,
        }
    }

    /// Get a mutable reference to a value from an array by index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to get.
    ///
    /// # Returns
    ///
    /// A mutable reference to the value at the given index, or `None` if this value
    /// is not an array or the index is out of bounds.
    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut ConfigValue> {
        match self {
            Self::Array(a) => a.get_mut(index),
            _ => None,
        }
    }

    /// Get a mutable reference to a value from a map by key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the value to get.
    ///
    /// # Returns
    ///
    /// A mutable reference to the value with the given key, or `None` if this value
    /// is not a map or the key does not exist.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut ConfigValue> {
        match self {
            Self::Map(m) => m.get_mut(key),
            _ => None,
        }
    }

    /// Set a value in a map.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set.
    /// * `value` - The value to set.
    ///
    /// # Returns
    ///
    /// `true` if the value was set successfully, `false` if this value is not a map.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<ConfigValue>) -> bool {
        match self {
            Self::Map(m) => {
                m.insert(key.into(), value.into());
                true
            }
            _ => false,
        }
    }

    /// Push a value onto an array.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to push.
    ///
    /// # Returns
    ///
    /// `true` if the value was pushed successfully, `false` if this value is not an array.
    pub fn push(&mut self, value: impl Into<ConfigValue>) -> bool {
        match self {
            Self::Array(a) => {
                a.push(value.into());
                true
            }
            _ => false,
        }
    }

    /// Remove a value from a map.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the value to remove.
    ///
    /// # Returns
    ///
    /// The removed value, or `None` if this value is not a map or the key does not exist.
    pub fn remove(&mut self, key: &str) -> Option<ConfigValue> {
        match self {
            Self::Map(m) => m.remove(key),
            _ => None,
        }
    }

    /// Remove a value from an array.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to remove.
    ///
    /// # Returns
    ///
    /// The removed value, or `None` if this value is not an array or the index is out of bounds.
    pub fn remove_index(&mut self, index: usize) -> Option<ConfigValue> {
        match self {
            Self::Array(a) if index < a.len() => Some(a.remove(index)),
            _ => None,
        }
    }

    /// Get the length of this value as an array or map.
    ///
    /// # Returns
    ///
    /// The length of the array or map, or `None` if this value is not an array or map.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::Array(a) => Some(a.len()),
            Self::Map(m) => Some(m.len()),
            _ => None,
        }
    }

    /// Check if this value as an array or map is empty.
    ///
    /// # Returns
    ///
    /// `true` if the array or map is empty, `false` if this value is not an array or map
    /// or the array or map is not empty.
    pub fn is_empty(&self) -> Option<bool> {
        match self {
            Self::Array(a) => Some(a.is_empty()),
            Self::Map(m) => Some(m.is_empty()),
            _ => None,
        }
    }

    /// Merge this value with another value.
    ///
    /// If both values are maps, the keys from the other map will be added to this map,
    /// overwriting any existing keys.
    /// If both values are arrays, the elements from the other array will be appended to this array.
    /// Otherwise, this value will be replaced with the other value.
    ///
    /// # Arguments
    ///
    /// * `other` - The value to merge with.
    pub fn merge(&mut self, other: ConfigValue) {
        match (self, other) {
            (Self::Map(a), Self::Map(b)) => {
                a.extend(b);
            }
            (Self::Array(a), Self::Array(b)) => {
                a.extend(b);
            }
            (a, b) => {
                *a = b;
            }
        }
    }
}

impl Default for ConfigValue {
    fn default() -> Self {
        Self::Null
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i32> for ConfigValue {
    fn from(i: i32) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<f32> for ConfigValue {
    fn from(f: f32) -> Self {
        Self::Float(f as f64)
    }
}

impl From<f64> for ConfigValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl<T: Into<ConfigValue>> From<Vec<T>> for ConfigValue {
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<ConfigValue>> From<HashMap<String, T>> for ConfigValue {
    fn from(m: HashMap<String, T>) -> Self {
        Self::Map(m.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "\"{}\"", s),
            Self::Array(a) => {
                write!(f, "[")?;
                let mut first = true;
                for v in a {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                    first = false;
                }
                write!(f, "]")
            }
            Self::Map(m) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in m {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            }
        }
    }
}

/// A configuration value with a path.
///
/// This structure represents a configuration value that can be
/// accessed using a path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// The root value.
    pub root: ConfigValue,
}

impl Config {
    /// Create a new configuration with a null root value.
    ///
    /// # Returns
    ///
    /// A new configuration with a null root value.
    pub fn new() -> Self {
        Self {
            root: ConfigValue::Null,
        }
    }

    /// Create a new configuration with a map root value.
    ///
    /// # Returns
    ///
    /// A new configuration with a map root value.
    pub fn new_map() -> Self {
        Self {
            root: ConfigValue::Map(HashMap::new()),
        }
    }

    /// Create a new configuration from a root value.
    ///
    /// # Arguments
    ///
    /// * `root` - The root value.
    ///
    /// # Returns
    ///
    /// A new configuration with the given root value.
    pub fn from_value(root: impl Into<ConfigValue>) -> Self {
        Self { root: root.into() }
    }

    /// Get a value by path.
    ///
    /// The path is a string of keys separated by dots.
    /// For example, `a.b.c` will get the value at key `c` in the map at key `b`
    /// in the map at key `a` in the root map.
    ///
    /// Array indices can be specified using square brackets, e.g. `a[0].b`.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the value.
    ///
    /// # Returns
    ///
    /// The value at the given path, or `None` if the path does not exist.
    pub fn get(&self, path: &str) -> Option<&ConfigValue> {
        let mut current = &self.root;

        if path.is_empty() {
            return Some(current);
        }

        for part in Config::parse_path(path) {
            match part {
                PathPart::Key(key) => {
                    current = current.get(key)?;
                }
                PathPart::Index(index) => {
                    current = current.get_index(index)?;
                }
            }
        }

        Some(current)
    }

    /// Get a value by path as a specific type.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the value.
    ///
    /// # Returns
    ///
    /// The value at the given path as the specified type, or `None` if the path
    /// does not exist or the value is not of the requested type.
    ///
    /// # Examples
    ///
    /// ```
    /// use lion_core::utils::Config;
    ///
    /// let mut config = Config::new_map();
    /// config.set("a.b.c", 42).expect("Failed to set value");
    ///
    /// let value = config.get_as::<i64>("a.b.c").expect("Failed to get value");
    /// assert_eq!(value, 42);
    ///
    /// let string_value = config.get_as::<String>("a.b.c");
    /// assert!(string_value.is_none()); // Not a string
    /// ```
    pub fn get_as<T: FromConfigValue>(&self, path: &str) -> Option<T> {
        self.get(path).and_then(T::from_config_value)
    }

    /// Get a mutable reference to a value by path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the value.
    ///
    /// # Returns
    ///
    /// A mutable reference to the value at the given path, or `None` if the path does not exist.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut ConfigValue> {
        let mut current = &mut self.root;

        if path.is_empty() {
            return Some(current);
        }

        // Parse the path first
        let parts = Self::parse_path(path);

        // Handle each part except the last one
        for i in 0..parts.len() - 1 {
            match parts[i] {
                PathPart::Key(key) => {
                    current = current.get_mut(key)?;
                }
                PathPart::Index(index) => {
                    current = current.get_index_mut(index)?;
                }
            }
        }

        // Handle the last part
        match parts.last().unwrap() {
            PathPart::Key(key) => current.get_mut(key),
            PathPart::Index(index) => current.get_index_mut(*index),
        }
    }

    /// Set a value by path.
    ///
    /// If the path does not exist, it will be created. This means that
    /// intermediate maps and arrays will be created as needed.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the value.
    /// * `value` - The value to set.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the value was set successfully, `Err(String)` with an error message if
    /// the path could not be created (e.g., trying to index into a non-array value).
    pub fn set(&mut self, path: &str, value: impl Into<ConfigValue>) -> Result<(), String> {
        if path.is_empty() {
            self.root = value.into();
            return Ok(());
        }

        let parts = Config::parse_path(path);
        let last_index = parts.len() - 1;

        let mut current = &mut self.root;

        for (i, part) in parts.iter().enumerate() {
            if i == last_index {
                // Last part, set the value
                match part {
                    PathPart::Key(key) => {
                        if !current.is_map() {
                            // Convert to map if not already
                            *current = ConfigValue::Map(HashMap::new());
                        }

                        if let Some(map) = current.as_map_mut() {
                            map.insert(key.to_string(), value.into());
                            return Ok(());
                        }
                    }
                    PathPart::Index(index) => {
                        if !current.is_array() {
                            // Convert to array if not already
                            *current = ConfigValue::Array(Vec::new());
                        }

                        if let Some(array) = current.as_array_mut() {
                            // Ensure the array is large enough
                            while array.len() <= *index {
                                array.push(ConfigValue::Null);
                            }

                            array[*index] = value.into();
                            return Ok(());
                        }
                    }
                }
            } else {
                // Intermediate part, ensure the path exists
                match part {
                    PathPart::Key(key) => {
                        if !current.is_map() {
                            // Convert to map if not already
                            *current = ConfigValue::Map(HashMap::new());
                        }

                        let map = match current.as_map_mut() {
                            Some(map) => map,
                            None => return Err(format!("Failed to access map at key '{}'", key)),
                        };

                        // Create the key if it doesn't exist
                        if !map.contains_key(&key.to_string()) {
                            let next_part = &parts[i + 1];
                            let new_value = match next_part {
                                PathPart::Key(_) => ConfigValue::Map(HashMap::new()),
                                PathPart::Index(_) => ConfigValue::Array(Vec::new()),
                            };
                            map.insert(key.to_string(), new_value);
                        }

                        current = map.get_mut(&key.to_string()).unwrap();
                    }
                    PathPart::Index(index) => {
                        if !current.is_array() {
                            // Convert to array if not already
                            *current = ConfigValue::Array(Vec::new());
                        }

                        let array = match current.as_array_mut() {
                            Some(array) => array,
                            None => {
                                return Err(format!("Failed to access array at index {}", index))
                            }
                        };

                        // Ensure the array is large enough
                        while array.len() <= *index {
                            let next_part = if i + 1 < parts.len() {
                                &parts[i + 1]
                            } else {
                                // This shouldn't happen, but just in case
                                return Err("Index out of bounds".to_string());
                            };

                            let new_value = match next_part {
                                PathPart::Key(_) => ConfigValue::Map(HashMap::new()),
                                PathPart::Index(_) => ConfigValue::Array(Vec::new()),
                            };
                            array.push(new_value);
                        }

                        current = array.get_mut(*index).unwrap();
                    }
                }
            }
        }

        // If we get here, something went wrong
        Err("Failed to set value".to_string())
    }

    /// Remove a value by path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the value to remove.
    ///
    /// # Returns
    ///
    /// The removed value, or `None` if the path does not exist.
    pub fn remove(&mut self, path: &str) -> Option<ConfigValue> {
        if path.is_empty() {
            let old_root = std::mem::replace(&mut self.root, ConfigValue::Null);
            return Some(old_root);
        }

        let parts = Config::parse_path(path);
        let last_index = parts.len() - 1;

        let mut current = &mut self.root;

        for (i, part) in parts.iter().enumerate() {
            if i == last_index {
                // Last part, remove the value
                return match part {
                    PathPart::Key(key) => current.remove(key),
                    PathPart::Index(index) => current.remove_index(*index),
                };
            }

            match part {
                PathPart::Key(key) => {
                    if let Some(next) = current.get_mut(key) {
                        current = next;
                    } else {
                        return None;
                    }
                }
                PathPart::Index(index) => {
                    if let Some(next) = current.get_index_mut(*index) {
                        current = next;
                    } else {
                        return None;
                    }
                }
            }
        }

        None
    }

    /// Check if a path exists in the configuration.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check.
    ///
    /// # Returns
    ///
    /// `true` if the path exists, `false` otherwise.
    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    /// Merge another configuration into this one.
    ///
    /// # Arguments
    ///
    /// * `other` - The configuration to merge.
    pub fn merge(&mut self, other: Config) {
        self.root.merge(other.root);
    }

    // Helper method to parse a path into parts
    fn parse_path<'a>(path: &'a str) -> Vec<PathPart<'a>> {
        let mut parts = Vec::new();
        let mut start = 0;
        let mut in_bracket = false;

        for (i, c) in path.char_indices() {
            match c {
                '.' if !in_bracket => {
                    if i > start {
                        parts.push(PathPart::Key(&path[start..i]));
                    }
                    start = i + 1;
                }
                '[' => {
                    if i > start {
                        parts.push(PathPart::Key(&path[start..i]));
                    }
                    start = i + 1;
                    in_bracket = true;
                }
                ']' if in_bracket => {
                    if i > start {
                        if let Ok(index) = path[start..i].parse::<usize>() {
                            parts.push(PathPart::Index(index));
                        }
                    }

                    // Skip the next dot if it exists
                    if i + 1 < path.len() && path[i + 1..].starts_with('.') {
                        start = i + 2;
                    } else {
                        start = i + 1;
                    }
                    in_bracket = false;
                }
                _ => {}
            }
        }

        // Add the last part if there is one
        if start < path.len() && !in_bracket {
            parts.push(PathPart::Key(&path[start..]));
        }

        parts
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)
    }
}

// Helper enum for path parsing
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathPart<'a> {
    Key(&'a str),
    Index(usize),
}

/// Trait for converting a ConfigValue to a specific type.
pub trait FromConfigValue: Sized {
    /// Convert a ConfigValue to this type.
    ///
    /// # Arguments
    ///
    /// * `value` - The ConfigValue to convert.
    ///
    /// # Returns
    ///
    /// The converted value, or `None` if the conversion is not possible.
    fn from_config_value(value: &ConfigValue) -> Option<Self>;
}

impl FromConfigValue for bool {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value.as_bool()
    }
}

impl FromConfigValue for i64 {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value.as_integer()
    }
}

impl FromConfigValue for f64 {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value.as_float()
    }
}

impl FromConfigValue for String {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value.as_str().map(String::from)
    }
}

impl<T: FromConfigValue> FromConfigValue for Vec<T> {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value
            .as_array()
            .map(|a| a.iter().filter_map(T::from_config_value).collect())
    }
}

impl<T: FromConfigValue> FromConfigValue for HashMap<String, T> {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        value.as_map().map(|m| {
            m.iter()
                .filter_map(|(k, v)| T::from_config_value(v).map(|v| (k.clone(), v)))
                .collect()
        })
    }
}

impl FromConfigValue for ConfigValue {
    fn from_config_value(value: &ConfigValue) -> Option<Self> {
        Some(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_types() {
        let null = ConfigValue::Null;
        assert!(null.is_null());

        let boolean = ConfigValue::Bool(true);
        assert!(boolean.is_bool());
        assert_eq!(boolean.as_bool(), Some(true));

        let integer = ConfigValue::Integer(42);
        assert!(integer.is_integer());
        assert!(integer.is_number());
        assert_eq!(integer.as_integer(), Some(42));
        assert_eq!(integer.as_float(), Some(42.0));

        let float = ConfigValue::Float(3.14);
        assert!(float.is_float());
        assert!(float.is_number());
        assert_eq!(float.as_float(), Some(3.14));

        let string = ConfigValue::String("hello".to_string());
        assert!(string.is_string());
        assert_eq!(string.as_str(), Some("hello"));

        let array = ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(2),
            ConfigValue::Integer(3),
        ]);
        assert!(array.is_array());
        assert_eq!(array.len(), Some(3));
        assert_eq!(array.is_empty(), Some(false));
        assert_eq!(array.get_index(1).unwrap().as_integer(), Some(2));

        let mut map = HashMap::new();
        map.insert("a".to_string(), ConfigValue::Integer(1));
        map.insert("b".to_string(), ConfigValue::String("hello".to_string()));
        let map_value = ConfigValue::Map(map);
        assert!(map_value.is_map());
        assert_eq!(map_value.len(), Some(2));
        assert_eq!(map_value.is_empty(), Some(false));
        assert_eq!(map_value.get("a").unwrap().as_integer(), Some(1));
        assert_eq!(map_value.get("b").unwrap().as_str(), Some("hello"));
    }

    #[test]
    fn test_config_value_from_types() {
        let bool_value: ConfigValue = true.into();
        assert_eq!(bool_value, ConfigValue::Bool(true));

        let int_value: ConfigValue = 42.into();
        assert_eq!(int_value, ConfigValue::Integer(42));

        let float_value: ConfigValue = 3.14.into();
        assert_eq!(float_value, ConfigValue::Float(3.14));

        let string_value: ConfigValue = "hello".into();
        assert_eq!(string_value, ConfigValue::String("hello".to_string()));

        let vec_value: ConfigValue = vec![1, 2, 3].into();
        assert_eq!(
            vec_value,
            ConfigValue::Array(vec![
                ConfigValue::Integer(1),
                ConfigValue::Integer(2),
                ConfigValue::Integer(3),
            ])
        );

        let mut map = HashMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);
        let map_value: ConfigValue = map.into();
        assert!(map_value.is_map());
        assert_eq!(map_value.get("a").unwrap().as_integer(), Some(1));
        assert_eq!(map_value.get("b").unwrap().as_integer(), Some(2));
    }

    #[test]
    fn test_config_value_mutations() {
        // Test array mutations
        let mut array = ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(2),
            ConfigValue::Integer(3),
        ]);

        // Push
        assert!(array.push(ConfigValue::Integer(4)));
        assert_eq!(array.len(), Some(4));
        assert_eq!(array.get_index(3).unwrap().as_integer(), Some(4));

        // Remove
        let removed = array.remove_index(1).unwrap();
        assert_eq!(removed.as_integer(), Some(2));
        assert_eq!(array.len(), Some(3));
        assert_eq!(array.get_index(1).unwrap().as_integer(), Some(3));

        // Test map mutations
        let mut map = ConfigValue::Map(HashMap::new());

        // Set
        assert!(map.set("a", ConfigValue::Integer(1)));
        assert!(map.set("b", ConfigValue::String("hello".to_string())));
        assert_eq!(map.len(), Some(2));

        // Get
        assert_eq!(map.get("a").unwrap().as_integer(), Some(1));
        assert_eq!(map.get("b").unwrap().as_str(), Some("hello"));

        // Get mut
        if let Some(value) = map.get_mut("a") {
            *value = ConfigValue::Integer(42);
        }
        assert_eq!(map.get("a").unwrap().as_integer(), Some(42));

        // Remove
        let removed = map.remove("a").unwrap();
        assert_eq!(removed.as_integer(), Some(42));
        assert_eq!(map.len(), Some(1));
        assert!(map.get("a").is_none());
    }

    #[test]
    fn test_config_value_merge() {
        // Test merging maps
        let mut map1 = ConfigValue::Map(HashMap::new());
        map1.set("a", 1);
        map1.set("b", 2);

        let mut map2 = ConfigValue::Map(HashMap::new());
        map2.set("b", 3);
        map2.set("c", 4);

        map1.merge(map2);

        assert_eq!(map1.get("a").unwrap().as_integer(), Some(1));
        assert_eq!(map1.get("b").unwrap().as_integer(), Some(3)); // Overwritten
        assert_eq!(map1.get("c").unwrap().as_integer(), Some(4));

        // Test merging arrays
        let mut array1 = ConfigValue::Array(vec![ConfigValue::Integer(1), ConfigValue::Integer(2)]);

        let array2 = ConfigValue::Array(vec![ConfigValue::Integer(3), ConfigValue::Integer(4)]);

        array1.merge(array2);

        assert_eq!(array1.len(), Some(4));
        assert_eq!(array1.get_index(0).unwrap().as_integer(), Some(1));
        assert_eq!(array1.get_index(1).unwrap().as_integer(), Some(2));
        assert_eq!(array1.get_index(2).unwrap().as_integer(), Some(3));
        assert_eq!(array1.get_index(3).unwrap().as_integer(), Some(4));

        // Test replacing
        let mut value = ConfigValue::Integer(1);
        value.merge(ConfigValue::String("hello".to_string()));
        assert_eq!(value.as_str(), Some("hello"));
    }

    #[test]
    fn test_config_paths() {
        let mut config = Config::new_map();

        // Set nested values
        config.set("a.b.c", 42).unwrap();
        config.set("a.d", "hello").unwrap();
        config.set("a.e[0]", 1).unwrap();
        config.set("a.e[1]", 2).unwrap();
        config.set("a.e[2]", 3).unwrap();

        // Get values
        assert_eq!(config.get("a.b.c").unwrap().as_integer(), Some(42));
        assert_eq!(config.get("a.d").unwrap().as_str(), Some("hello"));
        assert_eq!(config.get("a.e[0]").unwrap().as_integer(), Some(1));
        assert_eq!(config.get("a.e[1]").unwrap().as_integer(), Some(2));
        assert_eq!(config.get("a.e[2]").unwrap().as_integer(), Some(3));

        // Get non-existent values
        assert!(config.get("a.x").is_none());
        assert!(config.get("a.e[10]").is_none());

        // Get as specific types
        assert_eq!(config.get_as::<i64>("a.b.c"), Some(42));
        assert_eq!(config.get_as::<String>("a.d"), Some("hello".to_string()));
        assert!(config.get_as::<bool>("a.b.c").is_none()); // Not a boolean

        // Check if paths exist
        assert!(config.contains("a.b.c"));
        assert!(!config.contains("a.x"));

        // Remove values
        let removed = config.remove("a.b.c").unwrap();
        assert_eq!(removed.as_integer(), Some(42));
        assert!(!config.contains("a.b.c"));

        // Test path parsing with different formats
        let mut config = Config::new_map();
        config.set("a.b.c", 1).unwrap();
        config.set("a[0].b", 2).unwrap();
        config.set("a[1][0]", 3).unwrap();

        assert_eq!(config.get("a.b.c").unwrap().as_integer(), Some(1));
        assert_eq!(config.get("a[0].b").unwrap().as_integer(), Some(2));
        assert_eq!(config.get("a[1][0]").unwrap().as_integer(), Some(3));
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config::new_map();
        config1.set("a.b.c", 1).unwrap();
        config1.set("a.d", 2).unwrap();

        let mut config2 = Config::new_map();
        config2.set("a.b.c", 3).unwrap();
        config2.set("a.e", 4).unwrap();

        config1.merge(config2);

        assert_eq!(config1.get("a.b.c").unwrap().as_integer(), Some(3)); // Overwritten
        assert_eq!(config1.get("a.d").unwrap().as_integer(), Some(2));
        assert_eq!(config1.get("a.e").unwrap().as_integer(), Some(4));
    }

    #[test]
    fn test_config_from_config_value() {
        // Test primitive types
        assert_eq!(
            bool::from_config_value(&ConfigValue::Bool(true)),
            Some(true)
        );
        assert_eq!(i64::from_config_value(&ConfigValue::Integer(42)), Some(42));
        assert_eq!(
            f64::from_config_value(&ConfigValue::Float(3.14)),
            Some(3.14)
        );
        assert_eq!(
            String::from_config_value(&ConfigValue::String("hello".to_string())),
            Some("hello".to_string())
        );

        // Test array
        let array = ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(2),
            ConfigValue::Integer(3),
        ]);

        let vec: Option<Vec<i64>> = Vec::from_config_value(&array);
        assert_eq!(vec, Some(vec![1, 2, 3]));

        // Test map
        let mut map_value = ConfigValue::Map(HashMap::new());
        map_value.set("a", 1);
        map_value.set("b", 2);

        let map: Option<HashMap<String, i64>> = HashMap::from_config_value(&map_value);
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
    }

    #[test]
    fn test_config_value_display() {
        assert_eq!(ConfigValue::Null.to_string(), "null");
        assert_eq!(ConfigValue::Bool(true).to_string(), "true");
        assert_eq!(ConfigValue::Integer(42).to_string(), "42");
        assert_eq!(ConfigValue::Float(3.14).to_string(), "3.14");
        assert_eq!(
            ConfigValue::String("hello".to_string()).to_string(),
            "\"hello\""
        );

        let array = ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(2),
            ConfigValue::Integer(3),
        ]);
        assert_eq!(array.to_string(), "[1, 2, 3]");

        let mut map = ConfigValue::Map(HashMap::new());
        map.set("a", 1);
        map.set("b", "hello");

        // The order of keys in the map is not guaranteed, so we need to check both possibilities
        let display = map.to_string();
        assert!(
            display == "{\"a\": 1, \"b\": \"hello\"}" || display == "{\"b\": \"hello\", \"a\": 1}",
            "Unexpected display value: {}",
            display
        );
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::new_map();
        config.set("a.b.c", 42).unwrap();
        config.set("a.d", "hello").unwrap();
        config.set("a.e[0]", 1).unwrap();
        config.set("a.e[1]", 2).unwrap();

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            config.get("a.b.c").unwrap().as_integer(),
            deserialized.get("a.b.c").unwrap().as_integer()
        );
        assert_eq!(
            config.get("a.d").unwrap().as_str(),
            deserialized.get("a.d").unwrap().as_str()
        );
        assert_eq!(
            config.get("a.e[0]").unwrap().as_integer(),
            deserialized.get("a.e[0]").unwrap().as_integer()
        );
        assert_eq!(
            config.get("a.e[1]").unwrap().as_integer(),
            deserialized.get("a.e[1]").unwrap().as_integer()
        );
    }
}
