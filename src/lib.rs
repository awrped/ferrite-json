//! ferrite: json validator that tells you how to fix your mistakes
//!
//! ```ignore
//! use ferrite::validate_json;
//!
//! let json = r#"{"name": "Alice", "age": 30}"#;
//! match validate_json(json, "test.json".to_string(), 2) {
//!     Ok(_) => println!("Valid JSON!"),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

pub mod validator;

pub use validator::{validate_json, JsonError};