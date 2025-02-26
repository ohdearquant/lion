//! Configuration utilities.
//! 
//! This module provides utilities for configuration management.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// A configuration value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// Null value.
    Null,
    
    /// Boolean value.
    Boolean(bool),
    
    /// Integer value.
    Integer(i64),
    
    /// Float value.
    Float(f64),
    
    /// String value.
    String(String),
    
    /// Path value.
    Path(PathBuf),
    
    /// Array value.
    Array(Vec<ConfigValue>),
    
    /// Object value.
    Object(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    
    /// Check if this value is a boolean.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }
    
    /// Check if this value is an integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }
    
    /// Check if this value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }
    
    /// Check if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
    
    /// Check if this value is a path.
    pub fn is_path(&self) -> bool {
        matches!(self, Self::Path(_))
    }
    
    /// Check if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
    
    /// Check if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    
    /// Get this value as a boolean.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    
    /// Get this value as an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }
    
    /// Get this value as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }
    
    /// Get this value as a string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get this value as a path.
    pub fn as_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Path(p) => Some(p),
            _ => None,
        }
    }
    
    /// Get this value as an array.
    pub fn as_array(&self) -> Option<&[ConfigValue]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }
    
    /// Get this value as an object.
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
}

impl From<bool> for ConfigValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for ConfigValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for ConfigValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for ConfigValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for ConfigValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<PathBuf> for ConfigValue {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<Vec<ConfigValue>> for ConfigValue {
    fn from(value: Vec<ConfigValue>) -> Self {
        Self::Array(value)
    }
}

impl From<HashMap<String, ConfigValue>> for ConfigValue {
    fn from(value: HashMap<String, ConfigValue>) -> Self {
        Self::Object(value)
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(b) => write!(f, "{}", b),
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(
                Self::Path(p) => write!(f, "{}", p.display()),
                Self::Array(a) => {
                    write!(f, "[")?;
                    for (i, v) in a.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, "]")
                },
                Self::Object(o) => {
                    write!(f, "{{")?;
                    for (i, (k, v)) in o.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", k, v)?;
                    }
                    write!(f, "}}")
                },
            }
        }
    }
    
    #[cfg(test)]
    mod tests {
        use super::*;
        
        #[test]
        fn test_config_value_conversions() {
            // Test boolean
            let val = ConfigValue::from(true);
            assert!(val.is_boolean());
            assert_eq!(val.as_boolean(), Some(true));
            
            // Test integer
            let val = ConfigValue::from(42i64);
            assert!(val.is_integer());
            assert_eq!(val.as_integer(), Some(42));
            assert_eq!(val.as_float(), Some(42.0));
            
            // Test float
            let val = ConfigValue::from(3.14);
            assert!(val.is_float());
            assert_eq!(val.as_float(), Some(3.14));
            
            // Test string
            let val = ConfigValue::from("hello");
            assert!(val.is_string());
            assert_eq!(val.as_string(), Some("hello"));
            
            // Test path
            let val = ConfigValue::from(PathBuf::from("/tmp/file"));
            assert!(val.is_path());
            assert_eq!(val.as_path(), Some(&PathBuf::from("/tmp/file")));
            
            // Test array
            let val = ConfigValue::from(vec![
                ConfigValue::from(1i64),
                ConfigValue::from("hello"),
            ]);
            assert!(val.is_array());
            let array = val.as_array().unwrap();
            assert_eq!(array.len(), 2);
            assert_eq!(array[0].as_integer(), Some(1));
            assert_eq!(array[1].as_string(), Some("hello"));
            
            // Test object
            let mut map = HashMap::new();
            map.insert("key1".to_string(), ConfigValue::from(1i64));
            map.insert("key2".to_string(), ConfigValue::from("value"));
            let val = ConfigValue::from(map);
            assert!(val.is_object());
            let object = val.as_object().unwrap();
            assert_eq!(object.len(), 2);
            assert_eq!(object.get("key1").unwrap().as_integer(), Some(1));
            assert_eq!(object.get("key2").unwrap().as_string(), Some("value"));
        }
        
        #[test]
        fn test_config_value_display() {
            // Test null
            assert_eq!(ConfigValue::Null.to_string(), "null");
            
            // Test boolean
            assert_eq!(ConfigValue::from(true).to_string(), "true");
            assert_eq!(ConfigValue::from(false).to_string(), "false");
            
            // Test integer
            assert_eq!(ConfigValue::from(42i64).to_string(), "42");
            
            // Test float
            assert_eq!(ConfigValue::from(3.14).to_string(), "3.14");
            
            // Test string
            assert_eq!(ConfigValue::from("hello").to_string(), "hello");
            
            // Test path
            let path = PathBuf::from("/tmp/file");
            assert_eq!(ConfigValue::from(path.clone()).to_string(), path.display().to_string());
            
            // Test array
            let array = ConfigValue::from(vec![
                ConfigValue::from(1i64),
                ConfigValue::from("hello"),
            ]);
            assert_eq!(array.to_string(), "[1, hello]");
            
            // Test object
            let mut map = HashMap::new();
            map.insert("key1".to_string(), ConfigValue::from(1i64));
            map.insert("key2".to_string(), ConfigValue::from("value"));
            let object = ConfigValue::from(map);
            // Order is not guaranteed, so we need to check both possibilities
            let str = object.to_string();
            assert!(str == "{key1: 1, key2: value}" || str == "{key2: value, key1: 1}");
        }
    }
