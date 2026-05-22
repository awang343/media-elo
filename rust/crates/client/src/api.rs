use anyhow::{anyhow, Context, Result};
use media_elo_core::{
    AddRequest, AddTypeRequest, EditRequest, RenameTypeRequest, ReorderTypesRequest, Row,
    UndoRequest, VoteRequest, VoteResponse,
};
use reqwest::blocking::Client;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

pub struct Api {
    client: Client,
    base: String,
}

impl Api {
    pub fn new(base: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("building http client")?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub fn list_rows(&self) -> Result<Vec<Row>> {
        let resp = self
            .client
            .get(self.url("/rows"))
            .send()
            .context("GET /rows")?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn add_row(&self, req: &AddRequest) -> Result<Row> {
        self.post_json("/rows", req)
    }

    pub fn edit_row(&self, id: Uuid, req: &EditRequest) -> Result<Row> {
        let resp = self
            .client
            .put(self.url(&format!("/rows/{id}")))
            .json(req)
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn delete_row(&self, id: Uuid) -> Result<()> {
        self.client
            .delete(self.url(&format!("/rows/{id}")))
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn set_status(&self, id: Uuid, status: &str) -> Result<Row> {
        #[derive(Serialize)]
        struct Req<'a> {
            status: &'a str,
        }
        let resp = self
            .client
            .patch(self.url(&format!("/rows/{id}/status")))
            .json(&Req { status })
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn vote(&self, winner_id: Uuid, loser_id: Uuid) -> Result<VoteResponse> {
        self.post_json("/vote", &VoteRequest { winner_id, loser_id })
    }

    pub fn list_types(&self) -> Result<Vec<String>> {
        let resp = self
            .client
            .get(self.url("/types"))
            .send()
            .context("GET /types")?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn add_type(&self, name: &str) -> Result<Vec<String>> {
        let resp = self
            .client
            .post(self.url("/types"))
            .json(&AddTypeRequest {
                name: name.to_string(),
            })
            .send()?;
        json_or_err(resp)
    }

    pub fn delete_type(&self, name: &str) -> Result<()> {
        let resp = self
            .client
            .delete(self.url(&format!("/types/{}", urlencoding::encode(name))))
            .send()?;
        ok_or_err(resp)
    }

    pub fn rename_type(&self, old: &str, new_name: &str) -> Result<Vec<String>> {
        let resp = self
            .client
            .put(self.url(&format!("/types/{}", urlencoding::encode(old))))
            .json(&RenameTypeRequest {
                new_name: new_name.to_string(),
            })
            .send()?;
        json_or_err(resp)
    }

    pub fn reorder_types(&self, names: &[String]) -> Result<Vec<String>> {
        let resp = self
            .client
            .put(self.url("/types"))
            .json(&ReorderTypesRequest {
                names: names.to_vec(),
            })
            .send()?;
        json_or_err(resp)
    }

    pub fn undo(&self, req: &UndoRequest) -> Result<()> {
        let resp = self.client.post(self.url("/undo")).json(req).send()?;
        ok_or_err(resp)
    }

    fn post_json<B: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R> {
        let resp = self.client.post(self.url(path)).json(body).send()?;
        json_or_err(resp)
    }
}

/// Surface the server's `{"error": "..."}` body for non-2xx responses so the
/// UI shows the real message instead of reqwest's generic status text.
fn json_or_err<R: serde::de::DeserializeOwned>(resp: reqwest::blocking::Response) -> Result<R> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp.json()?)
    } else {
        Err(anyhow!(extract_error(resp, status)))
    }
}

fn ok_or_err(resp: reqwest::blocking::Response) -> Result<()> {
    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(anyhow!(extract_error(resp, status)))
    }
}

fn extract_error(resp: reqwest::blocking::Response, status: reqwest::StatusCode) -> String {
    #[derive(serde::Deserialize)]
    struct ErrBody {
        error: String,
    }
    match resp.json::<ErrBody>() {
        Ok(b) => b.error,
        Err(_) => format!("HTTP {status}"),
    }
}
