use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use shared::WalletEvent;
use std::time::Duration;

#[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
    topic: String,
}

impl KafkaProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self, rdkafka::error::KafkaError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("acks", "1")
            .create()?;

        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn publish(&self, event: &WalletEvent) -> Result<(), String> {
        let payload =
            serde_json::to_string(event).map_err(|e| format!("Failed to serialize event: {e}"))?;

        let key = match event {
            WalletEvent::WalletCreated { wallet_id, .. } => wallet_id.to_string(),
            WalletEvent::WalletFunded { wallet_id, .. } => wallet_id.to_string(),
            WalletEvent::TransferCompleted { from_wallet_id, .. } => from_wallet_id.to_string(),
            WalletEvent::TransferFailed { from_wallet_id, .. } => from_wallet_id.to_string(),
        };

        self.producer
            .send(
                FutureRecord::to(&self.topic).payload(&payload).key(&key),
                Timeout::After(Duration::from_secs(5)),
            )
            .await
            .map_err(|(e, _)| format!("Failed to publish event: {e}"))?;

        tracing::info!(
            event_type = ?event.event_type(),
            key = %key,
            topic = %self.topic,
            "Published wallet event"
        );

        Ok(())
    }
}
