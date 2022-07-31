mod orderbook {
    tonic::include_proto!("orderbook");
}

pub mod feeder;

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::Pair;
use tokio_stream::StreamExt;
use tonic::transport::Channel;

async fn get_summary(client: &mut OrderbookAggregatorClient<Channel>) {
    let mut stream = client
        .book_summary(Pair {
            pair: "ethbtc".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    while let Some(item) = stream.next().await {
        println!("{:?}", item.unwrap());
    }
    // stream is dropped here and the disconnect info is send to server
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderbookAggregatorClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();

    get_summary(&mut client).await;

    Ok(())
}
