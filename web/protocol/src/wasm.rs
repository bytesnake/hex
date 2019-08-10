use wasm_bindgen::prelude::*;
use bincode;

use objects::PacketId;
use objects::Request;
use objects::Answer;
use objects::RequestAction;
fn vec_to_id(buf: Vec<u32>) -> PacketId {
    [buf[0], buf[1], buf[2], buf[3]]
}

#[wasm_bindgen]
pub fn request_to_buf(id: Vec<u32>, msg: JsValue) -> Option<Vec<u8>> {
    let msg: RequestAction = msg.into_serde().ok()?;

    let req = Request::new(vec_to_id(id), msg);

    bincode::serialize(&req).ok()
}

#[wasm_bindgen]
pub fn upload_track(id: Vec<u32>, name: String, format: String, data: Vec<u8>) -> Option<Vec<u8>> {
    let msg = RequestAction::UploadTrack {
        name, format, data
    };

    let req = Request::new(vec_to_id(id), msg);

    bincode::serialize(&req).ok()
}

#[wasm_bindgen]
pub struct Wrapper(Option<Answer>);

#[wasm_bindgen]
impl Wrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(buf: Vec<u8>) -> Wrapper {
        Wrapper(bincode::deserialize(&buf).ok())
    }

    pub fn id(&self) -> Option<Vec<u32>> {
        if let Some(ref inner) = self.0 {
            let id = inner.id;
            Some(vec![id[0], id[1], id[2], id[3]])
        } else {
            None
        }
    }

    pub fn action(&self) -> JsValue {
        if let Some(ref inner) = self.0 {
            match inner.msg {
                Ok(ref answer) => {
                    JsValue::from_serde(&answer).unwrap_or(JsValue::null())
                },
                Err(ref err) => {
                    JsValue::from_str(&err)
                }
            }
        } else {
            JsValue::null()
        }
    }

    /*pub fn buffer(&self) -> Option<Vec<u8>> {
        if let Some(ref inner) = self.0 {
            match &inner.msg {
                AnswerAction::StreamNext(ref buf) => Some(buf.clone()),
                _ => None
            }
        } else {
            None
        }
    }*/
}
