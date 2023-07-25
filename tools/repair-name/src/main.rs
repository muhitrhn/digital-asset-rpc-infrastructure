mod account_updates;
mod ack;
pub mod config;
mod database;
pub mod error;
pub mod metrics;
mod program_transformers;
mod stream;
mod transaction_notifications;

use crate::{
    account_updates::account_worker,
    ack::ack_worker,
    config::{init_logger, setup_config, IngesterRole},
    database::setup_database,
    error::IngesterError,
    metrics::setup_metrics,
    stream::StreamSizeTimer,
    transaction_notifications::transaction_worker,
};


use cadence_macros::{is_global_default_set, statsd_count};
use chrono::Duration;
use log::{error, info};
use plerkle_messenger::{
    redis_messenger::RedisMessenger, ConsumptionType, ACCOUNT_STREAM, TRANSACTION_STREAM,
};
use tokio::{signal, task::JoinSet};

#[tokio::main(flavor = "multi_thread")]
pub async fn main() -> Result<(), IngesterError> {
    init_logger();
    info!("Starting nft_ingester");
    // Setup Configuration and Metrics ---------------------------------------------
    // Pull Env variables into config struct
    let config = setup_config();
    // Optionally setup metrics if config demands it
    setup_metrics(&config);
    // One pool many clones, this thing is thread safe and send sync
    let database_pool = setup_database(config.clone()).await;
    // The role determines the processes that get run.
    let role = config.clone().role.unwrap_or(IngesterRole::All);
    info!("Starting Program with Role {}", role);
    // Tasks Setup -----------------------------------------------
    // This joinset maages all the tasks that are spawned.
    let mut tasks = JoinSet::new();
    let stream_metrics_timer = Duration::seconds(30).to_std().unwrap();

    let mut timer_acc = StreamSizeTimer::new(
        stream_metrics_timer,
        config.messenger_config.clone(),
        ACCOUNT_STREAM,
    )?;
    let mut timer_txn = StreamSizeTimer::new(
        stream_metrics_timer.clone(),
        config.messenger_config.clone(),
        TRANSACTION_STREAM,
    )?;

    if let Some(t) = timer_acc.start::<RedisMessenger>().await {
        tasks.spawn(t);
    }
    if let Some(t) = timer_txn.start::<RedisMessenger>().await {
        tasks.spawn(t);
    }

    // Stream Consumers Setup -------------------------------------
    if role == IngesterRole::Ingester || role == IngesterRole::All {
        let (_ack_task, ack_sender) =
            ack_worker::<RedisMessenger>(config.get_messneger_client_config());
        for i in 0..config.get_account_stream_worker_count() {
            account_worker::<RedisMessenger>(
                database_pool.clone(),
                config.get_messneger_client_config(),
                ack_sender.clone(),
                if i == 0 {
                    ConsumptionType::Redeliver
                } else {
                    ConsumptionType::New
                },
            );
        }
        for i in 0..config.get_transaction_stream_worker_count() {
            transaction_worker::<RedisMessenger>(
                database_pool.clone(),
                config.get_messneger_client_config(),
                ack_sender.clone(),
                if i == 0 {
                    ConsumptionType::Redeliver
                } else {
                    ConsumptionType::New
                },
            );
        }
    }

    let roles_str = role.to_string();
    metric! {
        statsd_count!("ingester.startup", 1, "role" => &roles_str);
    }
    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    tasks.shutdown().await;

    Ok(())
}