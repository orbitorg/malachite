use std::future::Future;
use clap::Parser;
use tracing::info;

use crate::error::Error;
use malachite_app::Node;
use malachite_config::MetricsConfig;

use crate::metrics;

#[derive(Parser, Debug, Clone, Default, PartialEq)]
pub struct StartCmd {
    #[clap(long)]
    pub start_height: Option<u64>,
}

impl StartCmd {
    pub async fn run<N, F, Fut>(
        &self,
        node: &N,
        run: F,
        metrics: Option<MetricsConfig>,
    ) -> Result<(), Error>
    where
        N: Node,
        Fut: Future<Output = Result<(), Box<dyn core::error::Error>>> + Send,
        F: Fn(&N) -> Fut,
    {
        info!("Node is starting...");

        start(node, run, metrics).await.map_err(|error| Error::Runtime(error.to_string()))?;

        info!("Node has stopped");

        Ok(())
    }
}

/// start command to run a node.
pub async fn start<N, F, Fut>(
    node: &N,
    run: F,
    metrics: Option<MetricsConfig>,
) -> Result<(), Box<dyn core::error::Error>>
where
    N: Node,
    Fut: Future<Output = Result<(), Box<dyn core::error::Error>>> + Send,
    F: Fn(&N) -> Fut,
{
    // Enable Prometheus
    if let Some(metrics) = metrics {
        tokio::spawn(metrics::serve(metrics.clone()));
    }

    // Start the node
    run(node).await
}
