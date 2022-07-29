mod orderbook {
    tonic::include_proto!("orderbook");
}

//mod feeder;
mod feeder_new;

use orderbook::{Pair, Summary};
use std::net::SocketAddr;

use tokio::sync::mpsc::{self};

use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{transport::Server, Request, Response, Status};

use std::pin::Pin;

use crate::orderbook::orderbook_aggregator_server::{
    OrderbookAggregator, OrderbookAggregatorServer,
};

pub struct OrderbookAggregatorImpl {
    pub feeder: feeder_new::Feeder,
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorImpl {
    type BookSummaryStream = ResponseStream;

    async fn book_summary(
        &self,
        request: Request<Pair>, //Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        println!("Request from {:?}", request.remote_addr());

        let pair = request.get_ref().pair.clone();
        let mut stream = {
            self.feeder
                .channels
                .get(&pair)
                .unwrap()
                .lock()
                .unwrap()
                .subscribe()
        };

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(32);
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
            println!("\tclient disconnected: {:?}", request.remote_addr());
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::BookSummaryStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = feeder_new::Feeder::new();
    f.insert_channel("btcusdt");
    f.clone().watch_pair("btcusdt").await;

    println!("STARTING RPC SERVER");
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();

    let order_book = OrderbookAggregatorImpl { feeder: f };
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(order_book))
        .serve(addr)
        .await?;

    Ok(())
}
