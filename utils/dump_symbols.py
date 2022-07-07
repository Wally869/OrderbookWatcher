from binance.client import Client

api_key = "xxx"
api_secret = "xxx"

client = Client(api_key, api_secret)
exchange_info = client.get_exchange_info()
symbols = [s["symbol"] for s in exchange_info['symbols']]

with open("binance_symbols.txt", "w+") as f:
    for k, symbol in enumerate(symbols):
        if k < len(symbols) - 1:
            f.write('"' + symbol + '",')
        else:
            f.write('"' + symbol + '"')
