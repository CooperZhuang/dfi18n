use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

// The Tokio runtime for running asynchronous tasks
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

// Setup the Tokio runtime
pub fn setup() {
  RUNTIME.set(Builder::new_multi_thread().enable_all().build().unwrap()).unwrap();
}

// Gets the Tokio runtime
pub fn get() -> &'static Runtime {
  RUNTIME.get().unwrap()
}

// Spawn an asynchronous task on the Tokio runtime
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
  F: std::future::Future<Output = ()> + Send + 'static,
{
  get().spawn(future)
}
