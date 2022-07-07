use orderbook::{Empty, Level, Pair, Summary};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{self, Sender};

use tokio::sync::broadcast;

use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status};

use std::pin::Pin;

use std::time::Duration;

mod orderbook {
    tonic::include_proto!("orderbook");
}

mod binance;
mod feeder;

use crate::orderbook::orderbook_aggregator_server::{
    OrderbookAggregator, OrderbookAggregatorServer,
};

#[derive(Clone, Copy)]
pub enum FeederStatus {
    Uninitialized,
    Initializing,
    Running,
}

pub struct OrderbookAggregatorImpl {
    pub feeder_statuses: HashMap<String, Arc<Mutex<FeederStatus>>>,
    pub streams: HashMap<String, broadcast::Sender<Summary>>,
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorImpl {
    type BookSummaryStream = ResponseStream;

    async fn book_summary(
        &self,
        request: Request<Pair>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        println!("Request from {:?}", request.remote_addr());

        let target_pair = request.get_ref().pair.clone();
        let mut stream = self.streams.get(&target_pair).unwrap().subscribe();

        /*        // creating infinite stream with requested message
        let summy = Summary {
            spread: 0.01,
            bids: vec![Level {
                exchange: String::from("binance"),
                price: 0.57,
                amount: 1.23,
            }],
            asks: vec![Level {
                exchange: String::from("binance"),
                price: 0.57,
                amount: 1.23,
            }],
        };

        let repeat = std::iter::repeat(summy.clone());

        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));
        */

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            while let Ok(item) = stream.recv().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::BookSummaryStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let f = binance::Feeder::new();
    f.watch_pair_binance("btcusdt").await;

    println!("STARTING SERVER");
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();

    let (_tx, _rx) = mpsc::channel::<bool>(8);

    let bk = OrderbookAggregatorImpl {
        feeder_statuses: HashMap::new(),
        streams: HashMap::new(),
    };

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(bk))
        .serve(addr)
        .await?;

    Ok(())
}
