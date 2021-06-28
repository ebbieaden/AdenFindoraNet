use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Response<T> where T: Serialize {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> Response<T> {
    pub fn new_success(data: Option<T>) -> Self {
        Self {
            code: 0,
            message: String::from("success"),
            data,
        }
    }

    pub fn new_no_address() -> Self {
        Self {
            code: 90011,
            message: String::from("no address"),
            data: None,
        }
    }
}

