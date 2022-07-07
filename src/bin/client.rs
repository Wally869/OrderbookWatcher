mod orderbook {
    tonic::include_proto!("orderbook");
}

pub mod binance;

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::{Empty, Pair};
use tokio_stream::StreamExt;
use tonic::transport::Channel;

async fn get_summary(client: &mut OrderbookAggregatorClient<Channel>) {
    //let mut stream = client.book_summary(Empty {}).await.unwrap().into_inner();
    let mut stream = client
        .book_summary(Pair {
            pair: "btcusdt".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    // stream is infinite - take just 5 elements and then disconnect
    //let mut stream = stream.take(num);
    while let Some(item) = stream.next().await {
        println!("\treceived: {}", item.unwrap().spread);
    }
    // stream is droped here and the disconnect info is send to server
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderbookAggregatorClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();

    println!("Streaming echo:");
    //streaming_echo(&mut client).await;
    get_summary(&mut client).await;

    Ok(())
}
