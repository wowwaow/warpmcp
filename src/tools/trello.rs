use crate::utils::get_trello_config;
use anyhow::Result;
use serde_json::Value;

pub async fn get_board_lists(client: &reqwest::Client) -> Result<Vec<Value>> {
    let (key, token, board_id) = get_trello_config();
    
    let url = format!(
        "https://api.trello.com/1/boards/{}/lists?key={}&token={}",
        board_id, key, token
    );
    
    let lists: Vec<Value> = client.get(&url).send().await?.json().await?;
    Ok(lists)
}

pub async fn create_card(
    client: &reqwest::Client,
    list_id: &str,
    name: &str,
    desc: &str,
) -> Result<Value> {
    let (key, token, _) = get_trello_config();
    
    let url = format!(
        "https://api.trello.com/1/cards?key={}&token={}",
        key, token
    );
    
    let body = serde_json::json!({
        "idList": list_id,
        "name": name,
        "desc": desc
    });
    
    let card: Value = client.post(&url).json(&body).send().await?.json().await?;
    Ok(card)
}
