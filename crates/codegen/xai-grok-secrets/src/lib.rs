#![deny(clippy::indexing_slicing)]

mod sanitizer;

pub use sanitizer::{
    redact_json_string_values, redact_secrets, redact_url, redact_user_paths,
    redact_user_paths_with_backstop, walk_json_strings,
};
