use serde::de;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};
use serde_with::{serde_as, DisplayFromStr};

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

pub fn de_float_from_str<'a, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    str_val.parse::<f32>().map_err(de::Error::custom)
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

#[derive(Debug, Deserialize)]
struct Payload {
    #[serde(default)]
    values: Vec<Vec<Value>>,
}

#[derive(Debug)]
struct Value(f32);

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        s.parse().map(Value).map_err(|_| {
            D::Error::invalid_value(Unexpected::Str(s), &"a floating point number as a string")
        })
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderbook {
    pub last_update_id: u32,
    //#[serde(deserialize_with = "de_float_from_str")]
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub bids: Vec<Vec<f32>>,
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    #[serde(default)]
    pub asks: Vec<Vec<f32>>,
}
