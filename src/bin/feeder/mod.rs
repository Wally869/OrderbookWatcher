pub mod models;
//pub mod peers_data;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::mpsc;

use tungstenite::{connect, http::request};
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
        let pair = pair.to_string();
        let binance_url = format!("{}/ws/{}@depth@100ms", BINANCE_WS_API, pair.clone());
        let (mut socket, _response) =
            connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
        println!("Connected to binance stream for pair: {}", pair);

        let (tx, mut rx) = mpsc::channel::<DepthUpdateStreamData>(8);

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
                "https://api.binance.com/api/v3/depth?symbol={}&limit=1000",
                pair.clone()
            );
            let res = reqwest::get(target_url).await.unwrap();

            loop {
                while let Some(_msg) = rx.recv().await {
                    //println!("received message");
                }
            }
        });
        // query exchange for actual book
    }
}
