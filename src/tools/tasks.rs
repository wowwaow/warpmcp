use crate::schemas::*;
use crate::utils::{RedisManager, get_trello_config};
use anyhow::Result;
use std::time::Duration;
use crate::schemas::TrelloCard;
use crate::schemas::{TakeTaskArgs, UpdateTaskArgs};
use reqwest::Client;
use redis::AsyncCommands;
use serde_json::{json, Value};

pub async fn scan_trello_tasks(
    redis: &RedisManager,
    client: &reqwest::Client,
    args: Value,
) -> Result<String> {
    let (key, token, board_id) = get_trello_config();
    
    let url = format!(
        "https://api.trello.com/1/boards/{}/cards?key={}&token={}",
        board_id, key, token
    );
    
    let cards: Vec<TrelloCard> = match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                response.json().await?
            } else {
                return Err(anyhow::anyhow!("Trello API error: {}", response.status()));
            }
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to connect to Trello: {}", e))
    };
    
    // Filter by list if specified
    let _list_filter = args.get("list_filter").and_then(|v| v.as_str());
    
    // Get agent assignments from Redis
    let mut conn = redis.get_connection().await?;
    let mut enriched_cards = Vec::new();
    
    for card in cards {
        let assignment_key = format!("assignment:{}", card.id);
        let agent_id: Option<String> = conn.get(&assignment_key).await?;
        
        let status = if agent_id.is_some() {
            "assigned"
        } else {
            "available"
        };
        
        enriched_cards.push(json!({
            "id": card.id,
            "name": card.name,
            "description": card.desc,
            "list_id": card.id_list,
            "status": status,
            "assigned_to": agent_id,
            "url": card.url
        }));
    }
    
    Ok(json!({
        "cards": enriched_cards,
        "total": enriched_cards.len()
    }).to_string())
}

pub async fn take_trello_task(
    redis: &RedisManager,
    client: &reqwest::Client,
    args: Value,
) -> Result<String> {
    let params: TakeTaskArgs = serde_json::from_value(args)?;
    let mut conn = redis.get_connection().await?;
    
    // Check if task is already assigned
    let assignment_key = format!("assignment:{}", params.card_id);
    let existing: Option<String> = conn.get(&assignment_key).await?;
    
    if existing.is_some() {
        return Err(anyhow::anyhow!("Task already assigned to another agent"));
    }
    
    // Assign task
    let _: () = conn.set_ex(&assignment_key, &params.agent_id, 3600).await?;
    
    // Add to agent's active tasks
    let agent_tasks_key = format!("agent:{}:tasks", params.agent_id);
    let _: () = conn.sadd(&agent_tasks_key, &params.card_id).await?;
    
    // Add comment to Trello card
    let (key, token, _) = get_trello_config();
    let comment_url = format!(
        "https://api.trello.com/1/cards/{}/actions/comments?key={}&token={}",
        params.card_id, key, token
    );
    
    let comment_body = json!({
        "text": format!("Task claimed by agent: {}", params.agent_id)
    });
    
    // Add retry logic for Trello API calls
    let mut retries = 3;
    let mut delay = Duration::from_millis(100);
    
    loop {
        match client.post(&comment_url).json(&comment_body).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    break;
                } else {
                    if retries == 0 {
                        return Err(anyhow::anyhow!("Trello API error after retries: {}", response.status()));
                    }
                }
            }
            Err(e) => {
                if retries == 0 {
                    return Err(anyhow::anyhow!("Failed to connect to Trello after retries: {}", e));
                }
            }
        }
        
        retries -= 1;
        tokio::time::sleep(delay).await;
        delay *= 2; // Exponential backoff
    }
    
    Ok(format!("Task {} successfully assigned to agent {}", params.card_id, params.agent_id))
}

pub async fn update_trello_task(
    redis: &RedisManager,
    client: &reqwest::Client,
    args: Value,
) -> Result<String> {
    let params: UpdateTaskArgs = serde_json::from_value(args)?;
    let (key, token, _) = get_trello_config();
    
    match params.update_type.as_str() {
        "comment" => {
            let url = format!(
                "https://api.trello.com/1/cards/{}/actions/comments?key={}&token={}",
                params.card_id, key, token
            );
            
            let body = json!({
                "text": format!("[Agent {}] {}", params.agent_id, params.content)
            });
            
            client.post(&url).json(&body).send().await?;
        }
        "checklist" => {
            // Create or update checklist
            let url = format!(
                "https://api.trello.com/1/cards/{}/checklists?key={}&token={}",
                params.card_id, key, token
            );
            
            let body = json!({
                "name": "Progress",
                "pos": "bottom"
            });
            
            client.post(&url).json(&body).send().await?;
        }
        "description" => {
            let url = format!(
                "https://api.trello.com/1/cards/{}?key={}&token={}",
                params.card_id, key, token
            );
            
            let body = json!({
                "desc": params.content
            });
            
            client.put(&url).json(&body).send().await?;
        }
        "move_list" => {
            let url = format!(
                "https://api.trello.com/1/cards/{}?key={}&token={}",
                params.card_id, key, token
            );
            
            let body = json!({
                "idList": params.list_id.unwrap_or_default()
            });
            
            client.put(&url).json(&body).send().await?;
        }
        _ => return Err(anyhow::anyhow!("Invalid update type")),
    }
    
    // Store update in Redis for tracking
    let mut conn = redis.get_connection().await?;
    let update_key = format!("updates:{}:{}", params.card_id, chrono::Utc::now().timestamp());
    let update_data = json!({
        "agent_id": params.agent_id,
        "type": params.update_type,
        "content": params.content,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    let _: () = conn.set_ex(&update_key, update_data.to_string(), 86400 * 7).await?;
    
    Ok(format!("Task {} updated successfully", params.card_id))
}