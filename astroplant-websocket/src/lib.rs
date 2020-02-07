mod subscribers;
mod types;

use subscribers::Subscribers;
pub use types::RawMeasurement;

use jsonrpc_core::MetaIoHandler;
use jsonrpc_core::{futures, Params, Value};
use jsonrpc_pubsub::typed::{Sink, Subscriber};
use jsonrpc_pubsub::{PubSubHandler, Session, SubscriptionId}; //Sink, Subscriber, SubscriptionId};
use jsonrpc_server_utils::tokio;
use jsonrpc_ws_server::{RequestContext, ServerBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use futures::future::Future;

type PeripheralQuantityType = (i32, i32);

#[derive(Clone)]
struct WebSocketHandler {
    executor: tokio::runtime::TaskExecutor,
    raw_measurement_subscriptions: Arc<RwLock<HashMap<String, Subscribers<Sink<Value>>>>>, //HashMap<String, Vec<Sink>>>>,
    raw_measurement_buffer:
        Arc<RwLock<HashMap<String, HashMap<PeripheralQuantityType, RawMeasurement>>>>,
}

impl WebSocketHandler {
    fn new(executor: tokio::runtime::TaskExecutor) -> Self {
        Self {
            executor,
            raw_measurement_subscriptions: Arc::new(RwLock::new(HashMap::default())),
            raw_measurement_buffer: Arc::new(RwLock::new(HashMap::default())),
        }
    }

    fn buffer_raw_measurement(&self, kit_serial: String, raw_measurement: RawMeasurement) {
        let mut buffer = self.raw_measurement_buffer.write().unwrap();
        let index = (raw_measurement.peripheral, raw_measurement.quantity_type);

        buffer
            .entry(kit_serial)
            .or_default()
            .insert(index, raw_measurement);
    }

    fn publish_raw_measurement(&self, kit_serial: String, raw_measurement: RawMeasurement) {
        let subscriptions = self.raw_measurement_subscriptions.read().unwrap();

        let subscribers: Option<&Subscribers<Sink<Value>>> = subscriptions.get(&kit_serial);
        if let Some(subscribers) = subscribers {
            let value = serde_json::to_value(raw_measurement.clone()).unwrap();
            for subscriber in subscribers.values() {
                self.executor.spawn(
                    subscriber
                        .notify(Ok(value.clone()))
                        .map(|_| ())
                        .map_err(|_| ()),
                );
            }
        }

        self.buffer_raw_measurement(kit_serial, raw_measurement);
    }

    fn add_raw_measurement_subscriber(&self, kit_serial: String, subscriber: Subscriber<Value>) {
        let buffer = self.raw_measurement_buffer.read().unwrap();
        let resend: Vec<RawMeasurement> = match buffer.get(&kit_serial) {
            Some(pqt_raw_measurements) => pqt_raw_measurements.values().cloned().collect(),
            None => vec![],
        };

        let mut subscriptions = self.raw_measurement_subscriptions.write().unwrap();
        let subscribers = subscriptions.entry(kit_serial).or_default();
        let id = subscribers.add(subscriber);

        let sink = id.and_then(|id| subscribers.get(&id));

        // Resend buffered raw measurements to new connection.
        if let Some(sink) = sink {
            for raw_measurement in resend {
                self.executor.spawn(
                    sink.notify(Ok(serde_json::to_value(raw_measurement).unwrap()))
                        .map(|_| ())
                        .map_err(|_| ()),
                )
            }
        }
    }

    fn remove_raw_measurement_subscriber(&self, id: SubscriptionId) {}
}

pub struct WebSocketPublisher {
    // TODO: perhaps communicate through a channel if the RwLocks become a bottleneck
    handler: WebSocketHandler,
    server: jsonrpc_ws_server::Server,
}

impl WebSocketPublisher {
    pub fn publish_raw_measurement(&mut self, kit_serial: String, raw_measurement: RawMeasurement) {
        self.handler
            .publish_raw_measurement(kit_serial, raw_measurement);
    }
}

/// Runs a JSON-RPC server in a separate thread.
/// Returns a handle to publish to the server.
/// The server is stopped when the handle is dropped.
pub fn run() -> WebSocketPublisher {
    let mut runtime = tokio::runtime::Builder::new().build().unwrap();

    let handler = WebSocketHandler::new(runtime.executor());

    std::thread::spawn(move || runtime.block_on(futures::future::empty::<(), ()>()));

    let mut io = PubSubHandler::new(MetaIoHandler::default());
    io.add_subscription(
        "rawMeasurements",
        ("subscribe_rawMeasurements", {
            let handler = handler.clone();
            move |params: Params, _, subscriber: jsonrpc_pubsub::Subscriber| {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct SubParams {
                    kit_serial: String,
                }

                match params.parse::<SubParams>() {
                    Ok(sub_params) => {
                        let subscriber = Subscriber::new(subscriber);
                        handler
                            .clone()
                            .add_raw_measurement_subscriber(sub_params.kit_serial, subscriber);
                    }
                    Err(_) => {}
                }
            }
        }),
        ("unsubscribe_rawMeasurements", {
            let handler = handler.clone();

            move |id: SubscriptionId, _| {
                handler.clone().remove_raw_measurement_subscriber(id);
                futures::future::ok(Value::Bool(true))
            }
        }),
    );

    let server = ServerBuilder::with_meta_extractor(io, |context: &RequestContext| {
        Arc::new(Session::new(context.sender()))
    })
    .start(&"0.0.0.0:8081".parse().unwrap())
    .expect("could not start WS server");

    WebSocketPublisher {
        handler: handler.clone(),
        server,
    }
}
