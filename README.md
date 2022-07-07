# Orderbook Combiner   

Listen to websocket from Binance and Bitstamp to provide top 10 bids & asks from a merged orderbook.


## Run it 
```
cargo run --bin server
cargo run --bin client

```

## Process  
For a given pair:   
1- Open a ws stream to get depth change events on Binance and push them to a channel   
2- Query binance orderbook    
3- Update orderbook with depth change events already in channel and continue updating    
4- Push the top of the orderbook in another channel (let's call it mpsc-1)    

5- Open a ws stream to get orderbook from bitstamp    
6- Bitstamp provides partial orderbook from the ws so we just relay this orderbook in a channel (also pushing to mpsc-1)   

7- In another task we receive messages from this channel (mpsc-1) and create the aggregated orderbook    
8- Using the aggregated orderbook, we create a summary (top 10 bids and asks, spread)    
9- We push summary to a broadcast channel   

10- In the RPC implementation, we create a receiver by subscribing to an emitter of the broadcast channel   
11- RPC relays the summary to the client    


## Pairs available  
Bitstamp provides an easy to access list on its documentation. 
Binance's list can be accessed through its API.  

Using python and the python-binance pkg:  
```python  
from binance.client import Client

api_key = "xxx"
api_secret = "xxx"

client = Client(api_key, api_secret)
exchange_info = client.get_exchange_info()
for s in exchange_info['symbols']:
    print(s['symbol'])
```

Available pairs are registered in feeder/peers_data.rs  

## Issues  
The client does manage to get data, but the architecture seems impractical for several reasons:  
    - We get prices as f32, but we perform conversions to String to use them as keys for a HashMap (because floating points issues). This is costly and should be avoided  
    - Hashmap is used for the orderbook. A hashmap is unsorted and is not really appropriate. Orderbooks are usually better represented as trees or linked list. We could also use the next point  
    - We are passing a lot of data in the channels. Considering the aim, it would have been better to write orderbook in memory and use channels only to notify the receiver of changes done.   
In my opinion the best way to proceed would have been to write the orderbook using the levels struct fields to an in-memory relational database. This would have the added benefit of simplifying updates & queries; a simple SQL query would have returned the best non-zero bids & offer. A sharded database could also be a good possibility to handle different pairs & exchanges.    
    - The binance & bitstamp publishers seem to be fighting over data. It might have been better to go with a single publisher single consumer for both and handle combined orderbook creation using select!   


