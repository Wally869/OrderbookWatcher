pub mod models;
pub mod peers_data;

use itertools::Itertools;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::mpsc;

use tungstenite::{connect, http::request};
use url::Url;

use crate::binance::models::BinanceOrderbook;

use self::models::DepthUpdateStreamData;

static BINANCE_WS_API: &str = "wss://stream.binance.com:9443";
static BITSTAMP_WS_API: &str = "wss://ws.bitstamp.net/";

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

pub enum FeederStatus {
    Uninitialized,
    Initializing,
    Running,
}

#[derive(Clone)]
pub struct Feeder {
    pub books: HashMap<String, Arc<RwLock<Book>>>,
    pub status_tracker: HashMap<(String, String), Arc<Mutex<FeederStatus>>>,
}

impl Feeder {
    pub fn new() -> Self {
        return Feeder {
            books: HashMap::new(),
            status_tracker: HashMap::new(),
        };
    }

    pub async fn watch_pair(self, pair: &str) {
        self.clone().watch_pair_binance(pair).await;
        self.watch_pair_bitstamp(pair).await;

    }

    pub async fn watch_pair_bitstamp(self, pair: &str) {
        let pair = pair.to_string();
        let bitstamp_url = format!("{}", BITSTAMP_WS_API); //, pair.clone());      
        println!("{}", bitstamp_url);  
        let (mut socket, _response) =
        connect(Url::parse(&bitstamp_url).unwrap()).expect("Can't connect.");
        println!("Connected to bitstamp stream for pair: {}", pair);

        
        // Subscribe to Live Trades channel for BTC/USD
        socket
            .write_message(
                tungstenite::Message::Text(
                    json!({
                        "event": "bts:subscribe",
                        "data": {
                            "channel": format!("order_book_{}", pair)
                        }
                    })
                    .to_string(),
                )
                .into(),
            )
            .expect("Error sending message");

        // get first message (connection success)
        socket.read_message().unwrap();

        tokio::spawn(async move {
            loop {
                let msg = socket.read_message().expect("Error reading message");
                let msg = match msg {
                    tungstenite::Message::Text(s) => s,
                    _ => {
                        panic!("Error getting text");
                    }
                };
                
                // already ordered
                let parsed: models::WrapperBitstampOrderbook = serde_json::from_str(&msg).expect("Can't parse");
            }

        });
    }

    pub async fn watch_pair_binance(self, pair: &str) {
        let pair = pair.to_string();
        let binance_url = format!("{}/ws/{}@depth@100ms", BINANCE_WS_API, pair.clone());
        let (mut socket, _response) =
            connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
        println!("Connected to binance stream for pair: {}", pair);

        let (tx, mut rx) = mpsc::channel::<DepthUpdateStreamData>(32);

        tokio::spawn(async move {
            tokio::spawn(async move {
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

                    if let Err(_) = tx.send(parsed).await {
                        println!("receiver dropped");
                        return;
                    }
                }
            });

            // get initial state book
            let target_url = format!(
                "https://api.binance.com/api/v3/depth?symbol={}&limit=100",
                pair.clone().to_ascii_uppercase()
            );

            let res = reqwest::get(target_url)
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            let data_ob: BinanceOrderbook = serde_json::from_str(&res).unwrap();
            let last_update_id = data_ob.last_update_id.clone() as usize;

            let mut order_book_bids: HashMap<String, f32> = HashMap::new();
            let mut order_book_asks: HashMap<String, f32> = HashMap::new();

            for bid in data_ob.bids.into_iter() {
                order_book_bids.insert(bid[0].to_string(), bid[1]);
            }

            for ask in data_ob.asks.into_iter() {
                order_book_asks.insert(ask[0].to_string(), ask[1]);
            }

            loop {
                let mut modified: bool;
                while let Some(msg) = rx.recv().await {
                    modified = false;
                    if msg.u > last_update_id && msg.U > (last_update_id + 1) {
                        modified = true;
                        for ask in msg.a.into_iter() {
                            order_book_asks.insert(ask.price.to_string(), ask.size);
                        }

                        for bid in msg.b.into_iter() {
                            order_book_bids.insert(bid.price.to_string(), bid.size);
                        }
                    }

                    if modified {
                        let bids: Vec<(f32, f32)> = order_book_bids
                            .iter()
                            .sorted_by_key(|x| x.0)
                            .rev()
                            .filter(|(_, v)| v > &&0f32)
                            .take(10)
                            .map(|(k, v)| (k.to_owned().parse::<f32>().unwrap(), *v))
                            .collect();

                        let asks: Vec<(f32, f32)> = order_book_bids
                            .iter()
                            .sorted_by_key(|x| x.0)
                            .filter(|(_, v)| v > &&0f32)
                            .take(10)
                            .map(|(k, v)| (k.to_owned().parse::<f32>().unwrap(), *v))
                            .collect();
                    }
                }
            }
        });
        // query exchange for actual book
    }
}
