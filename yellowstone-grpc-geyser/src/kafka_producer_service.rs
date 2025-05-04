use {
    serde::Serialize,
    log::error,
    rdkafka::{
        producer::{
            FutureProducer,
            FutureRecord,
        },
        ClientConfig,
    },
    serde_json,
    tokio::sync::mpsc::{
        self,
        Receiver,
        Sender
    },
    std::time::Duration,
};

#[derive(Debug, Serialize)]
pub struct BillingEvent {
    pub team_id: String,
    pub app_id: String,
    pub eth_method: String,
    pub eth_network: String,
    pub subscription_id: String,
    pub subscription_type: String,
    pub log_source: String,
    pub response_content_length: u64,
}
#[derive(Debug, Serialize)]
struct KafkaPayload {
    namespace: String,
    records: Vec<KafkaRecord>,
}

#[derive(Debug, Serialize)]
struct KafkaRecord {
    #[serde(rename = "partitionKey")]
    partition_key: String,
    data: BillingEvent,
}

pub struct KafkaProducerService {
    pub sender: Sender<BillingEvent>,
}

impl KafkaProducerService {
    pub fn new(
        kafka_brokers: &str,
        kafka_username: Option<&str>,
        kafka_password: Option<&str>,
        kafka_topic: String,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx): (Sender<BillingEvent>, Receiver<BillingEvent>) = mpsc::channel(10000);

        let producer = Self::build_producer(kafka_brokers, kafka_username, kafka_password);

        let handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let kafka_payload = KafkaPayload {
                    namespace: "websocket-subscriptions".to_string(),
                    records: vec![KafkaRecord {
                        partition_key: format!("team-{}", event.team_id),
                        data: event,
                    }],
                };

                match serde_json::to_string(&kafka_payload) {
                    Ok(payload) => {
                        let record = FutureRecord::to(&kafka_topic)
                            .payload(&payload)
                            .key(&kafka_payload.records[0].partition_key);

                        match producer.send(record, Duration::from_secs(5)).await {
                            Ok(_) => {}
                            Err((e, _)) => error!("Kafka delivery failed: {:?}", e),
                        }
                    }
                    Err(e) => error!("Failed to serialize Kafka payload: {:?}", e),
                }
            }
        });

        (Self { sender: tx }, handle)
    }

    fn build_producer(
        brokers: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> FutureProducer {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", brokers)
            .set("compression.type", "gzip")
            .set("message.timeout.ms", "60000")
            .set("batch.num.messages", "1000")
            .set("linger.ms", "10");

        if let (Some(user), Some(pass)) = (username, password) {
            config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", "SCRAM-SHA-512")
                .set("sasl.username", user)
                .set("sasl.password", pass);
        }

        config
            .create()
            .expect("Failed to create Kafka FutureProducer")
    }
}
