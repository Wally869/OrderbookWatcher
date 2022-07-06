# Orderbook Combiner   

## Process  
For a given exchange and pair: 
1- Open a ws stream to get depth change events 
2- Parse messages and save events to an intermediary buffer / vec 
3- Query current orderbook 
4- Update orderbook by processing incoming depth change event  
5- Notify gRPC handler of orderbook change  

We do this for both exchanges.  
Handler / endpoint is notified through the use of an mpsc channel and dispatches new orderbook and midpoint to all listeners.  


## Notes  
Communication between exchange message receiver and orderbook manager is done through a Vec with an arc mutex lock. 
I could have used a channel instead, 