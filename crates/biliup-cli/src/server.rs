pub mod api;
pub mod app;
pub mod config;
pub mod core;
pub mod errors;
pub mod infrastructure;
mod router;
pub mod util;

use error_stack::ResultExt;
use futures::TryFutureExt;

use std::net::ToSocketAddrs;
// use tokio::sync::mpsc::Receiver;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::repositories;
use crate::server::infrastructure::service_register::ServiceRegister;
use tracing::info;
