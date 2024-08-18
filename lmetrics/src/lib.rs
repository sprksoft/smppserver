#[cfg(feature = "rocket")]
mod httpmetrics;
#[cfg(feature = "rocket")]
mod rocket;

mod nanohttp;

use std::{io::Write, net::TcpStream};

pub use once_cell;
use prometheus::{core::Collector, IntCounterVec, Opts, Registry, TextEncoder};

#[cfg(feature = "rocket")]
pub use {httpmetrics::*, rocket::*};

#[macro_export()]
macro_rules! register {
    ($($metric:path),+) => {
        {
            {
            let metrics = lmetrics::Metrics::new();
            $(metrics.register($metric.clone().into_collector());)+
            metrics
            }
        }
    };
}
#[macro_export()]
macro_rules! metrics {
    {$($vis:vis counter $name:ident ($help:literal, [$($label:ident),*]);)*} => {
        $(
        #[allow(dead_code)]
        #[allow(unused)]
        $vis mod $name {
            pub static METRIC: $crate::once_cell::sync::Lazy<$crate::Metric> = $crate::once_cell::sync::Lazy::new(|| {
                $crate::Metric::new(stringify!($name), $help, &[$(stringify!($label)),*])
            });
            pub fn inc($($label: &str,)*){
                METRIC.inc(&[$($label,)*]);
            }
        }
        )*
    };
}

#[derive(Clone)]
pub struct Metric {
    metric: IntCounterVec,
}
impl Metric {
    pub fn new(name: &str, help: &str, labels: &[&str]) -> Self {
        Self {
            metric: IntCounterVec::new(Opts::new(name, help), labels)
                .expect("Could not create counter"),
        }
    }
    pub fn inc(&self, labels: &[&str]) {
        self.metric.with_label_values(labels).inc();
    }

    pub fn into_collector(self) -> Box<dyn Collector> {
        Box::new(self.metric)
    }
}

#[derive(Clone)]
pub struct LMetrics<F>
where
    F: Fn() + Send + Sync + Clone,
{
    pub registry: Registry,
    before_handle: Option<F>,
}
impl<F: Fn() + Send + Sync + Clone> LMetrics<F> {
    pub fn new(metrics: &[&Metric]) -> Self {
        let me = Self::default();
        for met in metrics {
            me.register_metric(met);
        }
        me
    }
    pub fn register(&self, c: Box<dyn Collector>) {
        self.registry.register(c).unwrap();
    }
    pub fn register_metric(&self, metric: &Metric) {
        self.register(metric.clone().into_collector());
    }
    pub fn on_before_handle(&mut self, f: F) {
        self.before_handle = Some(f);
    }

    pub fn process_http_request(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let request = nanohttp::read_request(&mut stream)?;
        if request.starts_with("GET /metrics") {
            self.before_handle.as_ref().map(|e| e());
            let data = self.respond_metrics().unwrap();
            stream.write(data.as_bytes())?;
        } else {
            stream.write(nanohttp::respond_404().as_bytes())?;
        }
        Ok(())
    }

    pub fn accept(&self, listener: &mut std::net::TcpListener) -> std::io::Result<()> {
        match listener.accept() {
            Err(err) => match err.kind() {
                std::io::ErrorKind::WouldBlock => return Ok(()),
                _ => return Err(err),
            },
            Ok((stream, _)) => self.process_http_request(stream)?,
        }

        Ok(())
    }
    fn respond_metrics(&self) -> prometheus::Result<String> {
        let text_encoder = TextEncoder::new();
        let encoded = text_encoder.encode_to_string(&self.registry.gather())?;
        Ok(nanohttp::respond_200(encoded))
    }
}
impl<F: Fn() + Send + Sync + Clone> Default for LMetrics<F> {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
            before_handle: None,
        }
    }
}
