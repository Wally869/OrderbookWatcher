pub mod models;

use std::{
    sync::{Arc, Mutex},
};

use tungstenite::connect;
use url::Url;

use self::models::DepthUpdateStreamData;

static BINANCE_WS_API: &str = "wss://stream.binance.com:9443";

#[derive(Clone, Copy)]
pub struct LevelRecord {
    pub price: f32,
    pub quantity: f32,
}

#[derive(Clone)]
pub struct Book {
    pub bids: Vec<LevelRecord>,
    pub asks: Vec<LevelRecord>,
}

#[derive(Clone)]
pub struct BinanceFeeder {
    pub book: Book,
    pub buffer: Arc<Mutex<Vec<DepthUpdateStreamData>>>,
}

impl BinanceFeeder {
    pub fn new() -> Self {
        return BinanceFeeder {
            book: Book {
                bids: vec![],
                asks: vec![],
            },
            buffer: Arc::new(Mutex::new(vec![])),
        };
    }

    /// Query counterparty for full orderbook, then process depth updates
    pub async fn reconcile(self, _pair: &str) {
        tokio::spawn(async move {
            let buffer = self.buffer.clone();
            loop {
                {
                    let mut lock = buffer.lock().unwrap();

                    while let Some(_msg) = lock.pop() {}
                }
            }
        });
    }

    pub async fn watch_pair(self, pair: &str) {
        //-> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let binance_url = format!("{}/ws/{}@depth@100ms", BINANCE_WS_API, pair);
        let (mut socket, _response) =
            connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
        println!("Connected to binance stream.");

        let buffy = self.buffer.clone();
        tokio::spawn(async move {
            let buffer = buffy.clone();
            loop {
                let msg = socket.read_message().expect("Error reading message");
                let msg = match msg {
                    tungstenite::Message::Text(s) => s,
                    _ => {
                        panic!("Error getting text");
                    }
                };

                let parsed: models::DepthUpdateStreamData =
                    serde_json::from_str(&msg).expect("Can't parse");

                {
                    let mut lock = buffer.lock().unwrap();
                    lock.push(parsed);
                }
            }
        });
    }
}
