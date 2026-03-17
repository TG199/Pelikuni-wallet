use tracing::{subscriber::set_global_default, Subscriber};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry, fmt::MakeWriter};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where:
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = tracing_subscriber::fmt::layer()
        .with_writer(sink)
        .json();

    Registry::default()
        .with(env_filter)
        .with(formatting_layer)

}


pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    set_global_default(subscriber).expect("Failed to set subscriber");
}