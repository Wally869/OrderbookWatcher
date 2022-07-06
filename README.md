
// see process here: https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md  

1- Open stream to target   
2- Buffer events   
3- Query depth snapshot  
4- Query events too old compared to depth snapshot 
5- Keep processing events and notify main handler of new status 


