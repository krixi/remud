use cadence::{Counted, Gauged, NopMetricSink, StatsdClient, Timed, DEFAULT_PORT};
use once_cell::sync::OnceCell;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio_cadence::TokioBatchUdpMetricSink;

static METRICS: OnceCell<StatsdClient> = OnceCell::new();

async fn init_telegraf_metrics(host: &str) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let (sink, process) = TokioBatchUdpMetricSink::from((host, DEFAULT_PORT), socket)?;
    tokio::spawn(process);
    let client = StatsdClient::from_sink("remud", sink);
    METRICS.get_or_init(|| client);
    tracing::info!("initialized metrics client for host: {}", host);
    Ok(())
}

pub(crate) async fn init_metrics() {
    if init_telegraf_metrics("telegraf").await.is_ok() {
        tracing::info!("initialized metrics to host: telegraf");
    } else if init_telegraf_metrics("127.0.0.1").await.is_ok() {
        tracing::info!("initialized metrics to host: 127.0.0.1");
    } else {
        tracing::info!("using a no-op metrics client because telegraf is not available");
        let client = StatsdClient::from_sink("remud", NopMetricSink);
        METRICS.get_or_init(|| client);
    }
}

pub(crate) fn stats_time<'a, T: Into<&'a str>>(key: T, start: Instant) {
    if let Err(err) = METRICS
        .get()
        .unwrap()
        .time(key.into(), (Instant::now() - start).as_millis() as u64)
    {
        tracing::warn!("unable to post time: {:?}", err);
    }
}

pub(crate) fn stats_incr<'a, T: Into<&'a str>>(key: T) {
    if let Err(err) = METRICS.get().unwrap().incr(key.into()) {
        tracing::warn!("unable to post incr: {:?}", err);
    }
}

pub(crate) fn stats_gauge<'a, T: Into<&'a str>>(key: T, value: u64) {
    if let Err(err) = METRICS.get().unwrap().gauge(key.into(), value) {
        tracing::warn!("unable to post gauge: {:?}", err);
    }
}

// pub(crate) fn stats_decr<'a, T: Into<&'a str>>(key: T) {
//     if let Err(err) = METRICS.get().unwrap().decr(key.into()) {
//         tracing::warn!("unable to post decr: {:?}", err);
//     }
// }
//
// pub(crate) fn stats_histogram<'a, T: Into<&'a str>>(key: T, value: u64) {
//     if let Err(err) = METRICS.get().unwrap().histogram(key.into(), value) {
//         tracing::warn!("unable to post histogram: {:?}", err);
//     }
// }

pub(crate) struct StatsTimer<'a> {
    key: &'a str,
    start: Instant,
}
impl<'a> StatsTimer<'a> {
    pub fn new<T: Into<&'a str>>(key: T) -> Self {
        StatsTimer {
            key: key.into(),
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for StatsTimer<'a> {
    fn drop(&mut self) {
        stats_time(self.key, self.start);
    }
}
