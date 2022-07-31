pub mod models;
//pub mod peers_data;

use itertools::Itertools;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::{
    broadcast::Sender as broadcast_sender,
    mpsc::{self, Receiver},
};

use std::cmp::Ordering;

use tungstenite::connect;
use url::Url;

use crate::{
    feeder::models::BinanceOrderbook,
    orderbook::{Level, Summary},
};

use self::models::{DepthUpdateStreamData, WrapperOrderbook};

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

#[derive(Clone)]
pub struct Feeder {
    pub channels: HashMap<String, Arc<Mutex<broadcast_sender<Summary>>>>,
}

impl Feeder {
    pub fn new() -> Self {
        return Feeder {
            channels: HashMap::new(),
        };
    }

    pub fn insert_channel(&mut self, pair: &str) {
        let pair = pair.to_string().to_lowercase();

        {
            let (tx, _) = broadcast::channel::<Summary>(32);
            self.channels
                .insert(pair.clone(), Arc::new(Mutex::new(tx.clone())));
        }
    }

    pub async fn watch_pair(self, pair: &str) {
        let pair = pair.to_string().to_lowercase();

        tokio::task::spawn(async move {
            //let (tx, rx) = mpsc::channel::<WrapperOrderbook>(8);

            let recv_binance = self.clone().watch_pair_binance(pair.clone()).await;
            let recv_bitstamp = self.clone().watch_pair_bitstamp(pair.clone()).await;
            self.reconcile(pair, recv_binance, recv_bitstamp).await;
        });
    }

    /// From the channels orderbook data, compose the joint orderbook
    pub async fn reconcile(
        self,
        pair: String,
        mut receiver_binance: Receiver<WrapperOrderbook>,
        mut receiver_bitstamp: Receiver<WrapperOrderbook>,
    ) {
        //tokio::task::spawn(async move {
        let tx = { self.channels.get(&pair).unwrap().lock().unwrap().clone() };

        let mut binance_data: Option<WrapperOrderbook> = None;
        let mut bitstamp_data: Option<WrapperOrderbook> = None;

        //binance_data = Some(WrapperOrderbook{originator: Marketplace::Binance, bids: vec![], asks: vec![]});

        loop {
            tokio::select! {
                msg = receiver_binance.recv() => { /*println!("received from binance"); */ binance_data = msg },
                msg = receiver_bitstamp.recv() => { /*println!("received from bitstamp"); */  bitstamp_data = msg },
            };

            if binance_data.is_some() && bitstamp_data.is_some() {
                let binance = binance_data.clone().unwrap();
                let bitstamp = bitstamp_data.clone().unwrap();

                let mut bids: Vec<Level> = vec![];
                for bid in binance.bids {
                    bids.push(Level {
                        exchange: String::from("binance"),
                        price: bid[0].into(),
                        amount: bid[1].into(),
                    });
                }

                for bid in bitstamp.bids {
                    bids.push(Level {
                        exchange: String::from("bitstamp"),
                        price: bid[0].into(),
                        amount: bid[1].into(),
                    });
                }

                bids.sort_by(|a, b| {
                    if a.price < b.price {
                        Ordering::Less
                    } else if a.price == b.price {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                });
                bids = bids.into_iter().rev().take(20).collect();

                // now asks
                let mut asks: Vec<Level> = vec![];
                for ask in binance.asks {
                    asks.push(Level {
                        exchange: String::from("binance"),
                        price: ask[0].into(),
                        amount: ask[1].into(),
                    });
                }

                for ask in bitstamp.asks {
                    asks.push(Level {
                        exchange: String::from("bitstamp"),
                        price: ask[0].into(),
                        amount: ask[1].into(),
                    });
                }

                asks.sort_by(|a, b| {
                    if a.price < b.price {
                        Ordering::Less
                    } else if a.price == b.price {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                });
                //.rev()
                asks = asks.into_iter().take(20).collect();

                let summary = Summary {
                    spread: asks[0].price - bids[0].price,
                    bids: bids,
                    asks: asks,
                };

                if tx.receiver_count() > 0 {
                    tx.send(summary).unwrap();
                }
            }
        }
        //});
    }

    /// watch the given pair on bitstamp, and emit new orderbook on tx_orderbook
    pub async fn watch_pair_bitstamp(self, pair: String) -> Receiver<WrapperOrderbook> {
        //let pair = pair.to_string();
        let bitstamp_url = format!("{}", BITSTAMP_WS_API); //, pair.clone());
        let (mut socket, _response) =
            connect(Url::parse(&bitstamp_url).unwrap()).expect("Can't connect.");

        let (sender, receiver) = mpsc::channel::<WrapperOrderbook>(1);

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
        println!("Connected to bitstamp stream for pair: {}", pair);

        tokio::spawn(async move {
            loop {
                let msg = socket.read_message().expect("Error reading message");
                let msg = match msg {
                    tungstenite::Message::Text(s) => Some(s),
                    tungstenite::Message::Ping(payload) => {
                        socket
                            .write_message(tungstenite::Message::Pong(payload))
                            .unwrap();
                        println!("bitstamp: received ping, sent pong");
                        None
                    }
                    _ => {
                        panic!("Error getting text from bitstamp");
                    }
                };

                if let Some(msg) = msg {
                    // already ordered
                    let parsed: models::WrapperBitstampOrderbook =
                        serde_json::from_str(&msg).expect("Can't parse");

                    let ob = WrapperOrderbook {
                        originator: models::Marketplace::Bitstamp,
                        bids: parsed
                            .data
                            .bids
                            .into_iter()
                            .map(|elem| elem.into_iter().map(|elem| elem as f32).collect())
                            .collect(),
                        asks: parsed
                            .data
                            .asks
                            .into_iter()
                            .map(|elem| elem.into_iter().map(|elem| elem as f32).collect())
                            .collect(),
                    };

                    sender.send(ob).await.unwrap();
                }
            }
        });

        return receiver;
    }

    /// Watch the order book for a given pair on Binance.  
    /// Binance does not push partial books like Bitstamp so we start listening to changes in the orderbook
    /// then
    pub async fn watch_pair_binance(self, pair: String) -> Receiver<WrapperOrderbook> {
        let binance_url = format!("{}/ws/{}@depth@100ms", BINANCE_WS_API, pair.clone());
        let (mut socket, _response) =
            connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
        println!("Connected to binance stream for pair: {}", pair);

        let (sender, receiver) = mpsc::channel::<WrapperOrderbook>(1);

        let (tx, mut rx) = mpsc::channel::<DepthUpdateStreamData>(1);

        tokio::spawn(async move {
            loop {
                let msg = socket.read_message().expect("Error reading message");
                let msg = match msg {
                    tungstenite::Message::Text(s) => Some(s),
                    tungstenite::Message::Ping(payload) => {
                        socket
                            .write_message(tungstenite::Message::Pong(payload))
                            .unwrap();
                        println!("binance: received ping, sent pong");
                        None
                    }
                    _ => {
                        panic!("Error getting text from binance ws");
                    }
                };

                if let Some(msg) = msg {
                    let parsed: models::DepthUpdateStreamData =
                        serde_json::from_str(&msg).expect("Can't parse");

                    if let Err(_) = tx.send(parsed).await {
                        println!("receiver dropped");
                        return;
                    }
                }
            }
        });

        tokio::spawn(async move {
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
                        let bids: Vec<Vec<f32>> = order_book_bids
                            .iter()
                            .sorted_by(|a, b| {
                                let val_a = a.0.parse::<f32>().unwrap();
                                let val_b = b.0.parse::<f32>().unwrap();

                                if val_a < val_b {
                                    Ordering::Less
                                } else if val_a == val_b {
                                    Ordering::Equal
                                } else {
                                    Ordering::Greater
                                }
                            })
                            .rev()
                            .filter(|(_, v)| v > &&0f32)
                            .take(10)
                            .map(|(k, v)| vec![k.to_owned().parse::<f32>().unwrap(), *v])
                            .collect();

                        let asks: Vec<Vec<f32>> = order_book_asks
                            .iter()
                            .sorted_by(|a, b| {
                                let val_a = a.0.parse::<f32>().unwrap();
                                let val_b = b.0.parse::<f32>().unwrap();

                                if val_a < val_b {
                                    Ordering::Less
                                } else if val_a == val_b {
                                    Ordering::Equal
                                } else {
                                    Ordering::Greater
                                }
                            })
                            .filter(|(_, v)| v > &&0f32)
                            .take(10)
                            .map(|(k, v)| vec![k.to_owned().parse::<f32>().unwrap(), *v])
                            .collect();

                        let ob = WrapperOrderbook {
                            originator: models::Marketplace::Binance,
                            bids: bids,
                            asks: asks,
                        };

                        //tx_orderbook.send(ob).await.unwrap();
                        sender.send(ob).await.unwrap();
                    }
                }
            }
        });

        return receiver;
        // query exchange for actual book
    }
}
