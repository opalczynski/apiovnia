//! Per-entity repositories. Each repo is a stateless type with associated
//! functions that take `&SqlitePool` and return domain models. We use
//! associated functions (rather than methods on a struct) because there's
//! nothing to store; the type exists purely to namespace the SQL.

pub mod collections;
pub mod env_variables;
pub mod environments;
pub mod history;
pub mod overrides;
pub mod projects;
pub mod requests;
