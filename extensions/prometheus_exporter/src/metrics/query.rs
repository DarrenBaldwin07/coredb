use std::sync::{Arc, Mutex};

use pgx::bgworkers::*;
use pgx::prelude::*;
use pgx::{FromDatum, IntoDatum, PgOid};

pub const UPTIME_QUERY: &str =
    "SELECT FLOOR(EXTRACT(EPOCH FROM now() - pg_postmaster_start_time))::bigint
FROM pg_postmaster_start_time();";

// executes queries specifically for computation of metrics
// for now, only supports metrics which are updated/set with i64 values
pub fn handle_query(query: &str) -> Option<i64> {
    let uptime = Arc::new(Mutex::new(i64::default()));
    let clone = Arc::clone(&uptime);
    // interacting with the SPI bust be done in a background worker
    BackgroundWorker::transaction(move || {
        let mut obj_clone = clone.lock().unwrap();
        *obj_clone = query_exec(&query).unwrap();
    });
    let x = Some(*uptime.lock().unwrap());
    x
}

fn query_exec(query: &str) -> Option<i64> {
    Spi::get_one(&query)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use crate::metrics::query::{handle_query, query_exec, UPTIME_QUERY};
    use pgx::prelude::*;
    use pgx::bgworkers::*;
    use pgx::{FromDatum, IntoDatum, PgOid};
    use crate::background_worker;

    #[pg_test]
    fn test_query_exec() {
        assert!(query_exec(UPTIME_QUERY).is_some());
    }

    #[pg_test]
    fn test_handle_query() {
        // let arg = unsafe { i32::from_datum(arg, false) }.expect("invalid arg");
    
        let worker = BackgroundWorkerBuilder::new("dynamic_bgworker")
            .set_library("prometheus_exporter")
            .set_function("background_worker")
            .enable_spi_access()
            .set_notify_pid(unsafe { pg_sys::MyProcPid })
            .load_dynamic();
        let pid = worker.wait_for_startup().expect("no PID from the worker");
        assert!(pid > 0);
        assert!(handle_query(UPTIME_QUERY).is_some());
    }
}


// ps aux | grep -i Prometheus-Exporter | grep -v 'grep' |  awk '{print $2}'
// Neon is apache 2.0 licensed
// Supabase