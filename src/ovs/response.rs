use crate::error::VirtusError;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Response {
    id: i32,
    pub result: Vec<Entry>,
    pub error: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Entry {
    // TODO: error also gets parsed as Db
    Db(String),
    Rows { rows: Vec<Row> },
    // TODO: if Uuid is placed above ^Row, Row is never parsed. This should probably be ironed out.
    Uuid { uuid: Option<(String, String)> },
}

#[derive(Deserialize, Debug, Clone)]
pub struct Row {
    pub name: String,
    #[serde(rename = "_uuid")]
    pub uuid: (String, String),
}

impl TryFrom<Vec<u8>> for Response {
    type Error = VirtusError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        //println!("{}", String::from_utf8_lossy(&value));
        let value_str = std::str::from_utf8(&value)?;
        let response: Response = serde_json::from_str(value_str)?;
        //println!("{:?}", response);
        Ok(response)
    }
}
