mod remo;

use anyhow::{anyhow, Context as _, Result};
use clap::Parser;
use futures::prelude::*;
use remo::DeviceResponse;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use std::env;
use tokio::io;
use tokio::time::{interval, Duration};

fn main() -> Result<()> {
    let args = Args::parse();

    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::select! {
                r = signal() => r.context("failed to receive signal"),
                r = async_main(args) => r,
            }
        })
}

#[cfg(unix)]
async fn signal() -> io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = interrupt.recv() => {}
        _ = terminate.recv() => {}
    }

    Ok(())
}

#[cfg(windows)]
async fn signal() -> io::Result<()> {
    use tokio::signal::windows::{ctrl_break, ctrl_c};

    let mut ctrl_c = ctrl_c()?;
    let mut ctrl_break = ctrl_break()?;

    tokio::select! {
        _ = ctrl_c.recv() => {}
        _ = ctrl_break.recv() => {}
    }

    Ok(())
}

#[cfg(not(any(unix, windows)))]
async fn signal() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    interval: u64,
}

async fn async_main(args: Args) -> Result<()> {
    let ctx = Context::new()?;

    let mut int = interval(Duration::from_secs(args.interval));
    loop {
        int.tick().await;
        ctx.poll().and_then(|devs| ctx.write(devs)).await?;
    }
}

struct Context {
    remo: reqwest::Client,
    db: reqwest::Client,
    influx_url: reqwest::Url,
    influx_bucket: String,
    influx_org: String,
}

impl Context {
    fn new() -> Result<Self> {
        let remo_token =
            env::var("NATURE_REMO_API_TOKEN").context("`NATURE_REMO_API_TOKEN` is not set")?;

        let influx_url = env::var("INFLUX_URL").context("`INFLUX_URL` is not set")?;
        let influx_bucket = env::var("INFLUX_BUCKET").context("`INFLUX_BUCKET` is not set")?;
        let influx_org = env::var("INFLUX_ORG").context("`INFLUX_ORG` is not set")?;

        let influx_api_token = env::var("INFLUX_API_TOKEN").ok();
        let influx_username = env::var("INFLUX_USERNAME").ok();
        let influx_password = env::var("INFLUX_PASSWORD").ok();
        let influx_auth = if let Some(token) = influx_api_token {
            format!("Token {token}")
        } else if influx_username.is_some() && influx_password.is_some() {
            use base64::prelude::*;
            format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!(
                    "{}:{}",
                    influx_username.unwrap(),
                    influx_password.unwrap()
                ))
            )
        } else {
            return Err(anyhow!("either `INFLUX_API_TOKEN` or `INFLUX_USERNAME` and `INFLUX_PASSWORD` must be defined"));
        };

        let remo = {
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                format!("Bearer {remo_token}").parse().unwrap(),
            );
            reqwest::Client::builder()
                .default_headers(headers)
                .build()?
        };

        let db = {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, influx_auth.parse().unwrap());
            reqwest::Client::builder()
                .default_headers(headers)
                .build()?
        };

        Ok(Self {
            remo,
            db,
            influx_url: influx_url.parse()?,
            influx_bucket,
            influx_org,
        })
    }

    async fn poll(&self) -> Result<Vec<DeviceResponse>> {
        self.remo
            .get("https://api.nature.global/1/devices")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("failed to get devices")
    }

    async fn write(&self, devs: Vec<DeviceResponse>) -> Result<()> {
        let body = devs
            .into_iter()
            .flat_map(|d| d.into_write_query())
            .collect::<Vec<_>>()
            .join("\n");

        let mut url = self.influx_url.join("/api/v2/write")?;
        url.query_pairs_mut()
            .append_pair("bucket", self.influx_bucket.as_str())
            .append_pair("org", self.influx_org.as_str())
            .append_pair("precision", "s");

        self.db
            .post(url)
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
