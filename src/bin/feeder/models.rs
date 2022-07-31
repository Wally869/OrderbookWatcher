use serde::de;

use serde::{Deserialize, Deserializer};
use serde_with::{serde_as, DisplayFromStr};

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum Marketplace {
    Binance,
    Bitstamp,
}

impl Marketplace {
    pub fn to_string(self) -> String {
        match self {
            Marketplace::Binance => String::from("binance"),
            Marketplace::Bitstamp => String::from("bitstamp"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct WrapperOrderbook {
    pub originator: Marketplace,
    pub bids: Vec<Vec<f32>>,
    pub asks: Vec<Vec<f32>>,
}

pub fn de_float_from_str<'a, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    str_val.parse::<f32>().map_err(de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct WrapperBitstampOrderbook {
    pub data: BitstampOrderbook,
    pub channel: String,
    pub event: String,
}

// price - quantity for orders
#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct BitstampOrderbook {
    pub timestamp: String,
    pub microtimestamp: String,
    //#[serde(deserialize_with = "de_float_from_str")]
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub bids: Vec<Vec<f64>>,
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub asks: Vec<Vec<f64>>,
    //pub channel: String,
    //pub event: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OfferData {
    #[serde(deserialize_with = "de_float_from_str")]
    pub price: f32,
    #[serde(deserialize_with = "de_float_from_str")]
    pub size: f32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthStreamData {
    pub last_update_id: usize,
    pub bids: Vec<OfferData>,
    pub asks: Vec<OfferData>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
// #[serde(rename_all = "camelCase")]
pub struct DepthUpdateStreamData {
    pub e: String,
    pub E: usize,
    pub s: String,
    pub U: usize,
    pub u: usize,
    pub b: Vec<OfferData>,
    pub a: Vec<OfferData>,
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderbook {
    pub last_update_id: u64,
    //#[serde(deserialize_with = "de_float_from_str")]
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub bids: Vec<Vec<f32>>,
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub asks: Vec<Vec<f32>>,
}
