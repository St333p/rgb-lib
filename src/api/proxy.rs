use amplify::s;
use reqwest::blocking::{multipart, Client};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

use crate::error::{Error, InternalError};

const JSON: &str = "application/json";

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub(crate) code: i64,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest<P> {
    method: String,
    jsonrpc: String,
    id: Option<String>,
    params: Option<P>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcResponse<R> {
    id: Option<String>,
    pub(crate) result: Option<R>,
    pub(crate) error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NullRequest;

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerInfoResponse {
    pub(crate) protocol_version: String,
    pub(crate) version: String,
    pub(crate) uptime: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AckResponse {
    pub(crate) success: bool,
    pub(crate) ack: Option<bool>,
    pub(crate) nack: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetConsignmentResponse {
    pub(crate) consignment: String,
    pub(crate) txid: String,
    pub(crate) vout: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostAckParams {
    recipient_id: String,
    ack: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostConsignmentParams {
    recipient_id: String,
    txid: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostConsignmentWithVoutParams {
    recipient_id: String,
    txid: String,
    vout: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecipientIDParam {
    recipient_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachmentIdParam {
    attachment_id: String,
}

pub trait Proxy {
    fn get_info(self, url: &str) -> Result<JsonRpcResponse<ServerInfoResponse>, Error>;

    fn get_ack(self, url: &str, recipient_id: String) -> Result<JsonRpcResponse<bool>, Error>;

    fn get_consignment(
        self,
        url: &str,
        recipient_id: String,
    ) -> Result<JsonRpcResponse<GetConsignmentResponse>, Error>;

    fn get_media(self, url: &str, attachment_id: String) -> Result<JsonRpcResponse<String>, Error>;

    fn post_ack(
        self,
        url: &str,
        recipient_id: String,
        ack: bool,
    ) -> Result<JsonRpcResponse<bool>, Error>;

    fn post_consignment(
        self,
        url: &str,
        recipient_id: String,
        consignment_path: PathBuf,
        txid: String,
        vout: Option<u32>,
    ) -> Result<JsonRpcResponse<bool>, Error>;

    fn post_media(
        self,
        url: &str,
        attachment_id: String,
        media_path: PathBuf,
    ) -> Result<JsonRpcResponse<bool>, Error>;
}

impl Proxy for Client {
    fn get_info(self, url: &str) -> Result<JsonRpcResponse<ServerInfoResponse>, Error> {
        let body: JsonRpcRequest<NullRequest> = JsonRpcRequest {
            method: s!("server.info"),
            jsonrpc: s!("2.0"),
            id: None,
            params: None,
        };
        Ok(self
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .json(&body)
            .send()?
            .json::<JsonRpcResponse<ServerInfoResponse>>()
            .map_err(InternalError::from)?)
    }

    fn get_ack(self, url: &str, recipient_id: String) -> Result<JsonRpcResponse<bool>, Error> {
        let body = JsonRpcRequest {
            method: s!("ack.get"),
            jsonrpc: s!("2.0"),
            id: None,
            params: Some(RecipientIDParam { recipient_id }),
        };
        Ok(self
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .json(&body)
            .send()?
            .json::<JsonRpcResponse<bool>>()
            .map_err(InternalError::from)?)
    }

    fn get_consignment(
        self,
        url: &str,
        recipient_id: String,
    ) -> Result<JsonRpcResponse<GetConsignmentResponse>, Error> {
        let body = JsonRpcRequest {
            method: s!("consignment.get"),
            jsonrpc: s!("2.0"),
            id: None,
            params: Some(RecipientIDParam { recipient_id }),
        };
        Ok(self
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .json(&body)
            .send()?
            .json::<JsonRpcResponse<GetConsignmentResponse>>()
            .map_err(InternalError::from)?)
    }

    fn get_media(self, url: &str, attachment_id: String) -> Result<JsonRpcResponse<String>, Error> {
        let body = JsonRpcRequest {
            method: s!("media.get"),
            jsonrpc: s!("2.0"),
            id: None,
            params: Some(AttachmentIdParam { attachment_id }),
        };
        Ok(self
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .json(&body)
            .send()?
            .json::<JsonRpcResponse<String>>()
            .map_err(InternalError::from)?)
    }

    fn post_ack(
        self,
        url: &str,
        recipient_id: String,
        ack: bool,
    ) -> Result<JsonRpcResponse<bool>, Error> {
        let body = JsonRpcRequest {
            method: s!("ack.post"),
            jsonrpc: s!("2.0"),
            id: None,
            params: Some(PostAckParams { recipient_id, ack }),
        };
        Ok(self
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .json(&body)
            .send()?
            .json::<JsonRpcResponse<bool>>()
            .map_err(InternalError::from)?)
    }

    fn post_consignment(
        self,
        url: &str,
        recipient_id: String,
        consignment_path: PathBuf,
        txid: String,
        vout: Option<u32>,
    ) -> Result<JsonRpcResponse<bool>, Error> {
        let params = if let Some(vout) = vout {
            serde_json::to_string(&PostConsignmentWithVoutParams {
                recipient_id,
                txid,
                vout,
            })
            .map_err(InternalError::from)?
        } else {
            serde_json::to_string(&PostConsignmentParams { recipient_id, txid })
                .map_err(InternalError::from)?
        };
        let form = multipart::Form::new()
            .text("method", "consignment.post")
            .text("jsonrpc", "2.0")
            .text("id", "1")
            .text("params", params)
            .file("file", consignment_path)?;
        Ok(self
            .post(url)
            .multipart(form)
            .send()?
            .json::<JsonRpcResponse<bool>>()
            .map_err(InternalError::from)?)
    }

    fn post_media(
        self,
        url: &str,
        attachment_id: String,
        media_path: PathBuf,
    ) -> Result<JsonRpcResponse<bool>, Error> {
        let form = multipart::Form::new()
            .text("method", "media.post")
            .text("jsonrpc", "2.0")
            .text("id", "1")
            .text(
                "params",
                serde_json::to_string(&AttachmentIdParam { attachment_id })
                    .map_err(InternalError::from)?,
            )
            .file("file", media_path)?;
        Ok(self
            .post(url)
            .multipart(form)
            .send()?
            .json::<JsonRpcResponse<bool>>()
            .map_err(InternalError::from)?)
    }
}
