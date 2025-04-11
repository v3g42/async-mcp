use std::sync::{Arc, Mutex};

use async_mcp::{
    server::{Server, ServerBuilder},
    transport::ServerStdioTransport,
    types::{CallToolRequest, CallToolResponse, ServerCapabilities, Tool, ToolResponseContent},
};
use serde_json::json;
use types::{AddObservationParams, DeleteObservationParams, Entity, KnowledgeGraph, Relation};

use anyhow::Result;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // needs to be stderr due to stdio transport
        .with_writer(std::io::stderr)
        .init();

    let mut server = Server::builder(ServerStdioTransport).capabilities(ServerCapabilities {
        tools: Some(json!({})),
        ..Default::default()
    });
    register_tools(&mut server)?;

    let server = server.build();
    server
        .listen()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    Ok(())
}

fn register_tools(server: &mut ServerBuilder<ServerStdioTransport>) -> Result<()> {
    let memory_file_path = "kb_memory.json";
    let kg = KnowledgeGraph::load_from_file(memory_file_path)?;
    let kg = Arc::new(Mutex::new(kg));

    let description = Tool {
        name: "create_entities".to_string(),
        description: Some("Create multiple new entities".to_string()),
        input_schema: json!({
           "type":"object",
           "properties":{
              "entities":{
                 "type":"array",
                 "items":{
                    "type":"object",
                    "properties":{
                       "name":{"type":"string"},
                       "entityType":{"type":"string"},
                       "observations":{
                          "type":"array", "items":{"type":"string"}
                       }
                    },
                    "required":["name","entityType","observations"]
                 }
              }
           },
           "required":["entities"]
        }),
        output_schema: None,
    };

    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let entities = args
                .get("entities")
                .ok_or(anyhow::anyhow!("missing arguments `entities`"))?;
            let entities: Vec<Entity> = serde_json::from_value(entities.clone())?;
            let created = kg_clone.lock().unwrap().create_entities(entities)?;
            kg_clone.lock().unwrap().save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(created).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "create_relations".to_string(),
        description: Some("Create multiple new relations".to_string()),
        input_schema: json!({
           "type":"object",
           "properties":{
              "relations":{
                 "type":"array",
                 "items":{
                    "type":"object",
                    "properties":{
                       "from":{"type":"string"},
                       "to":{"type":"string"},
                       "relationType":{"type":"string"}
                    },
                    "required":["from","to","relationType"]
                 }
              }
           },
           "required":["relations"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let relations = args
                .get("relations")
                .ok_or(anyhow::anyhow!("missing arguments `relations`"))?;
            let relations: Vec<Relation> = serde_json::from_value(relations.clone())?;
            let created = kg_clone.lock().unwrap().create_relations(relations)?;
            kg_clone.lock().unwrap().save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(created).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "add_observations".to_string(),
        description: Some("Add new observations to existing entities".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "observations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "entityName": {"type": "string"},
                            "contents": {
                                "type": "array",
                                "items": {"type": "string"}
                            }
                        },
                        "required": ["entityName", "contents"]
                    }
                }
            },
            "required": ["observations"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let observations = args
                .get("observations")
                .ok_or(anyhow::anyhow!("missing arguments `observations`"))?;
            let observations: Vec<AddObservationParams> =
                serde_json::from_value(observations.clone())?;
            let results = kg_clone.lock().unwrap().add_observations(observations)?;
            kg_clone.lock().unwrap().save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(results).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "delete_entities".to_string(),
        description: Some("Delete multiple entities and their relations".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "entityNames": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["entityNames"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let entity_names = args
                .get("entityNames")
                .ok_or(anyhow::anyhow!("missing arguments `entityNames`"))?;
            let entity_names: Vec<String> = serde_json::from_value(entity_names.clone())?;
            let mut kg_guard = kg_clone.lock().unwrap();
            kg_guard.delete_entities(entity_names)?;
            kg_guard.save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: "Entities deleted successfully".to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "delete_observations".to_string(),
        description: Some("Delete specific observations from entities".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "deletions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "entityName": {"type": "string"},
                            "observations": {
                                "type": "array",
                                "items": {"type": "string"}
                            }
                        },
                        "required": ["entityName", "observations"]
                    }
                }
            },
            "required": ["deletions"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let deletions = args
                .get("deletions")
                .ok_or(anyhow::anyhow!("missing arguments `deletions`"))?;
            let deletions: Vec<DeleteObservationParams> =
                serde_json::from_value(deletions.clone())?;
            let mut kg_guard = kg_clone.lock().unwrap();
            kg_guard.delete_observations(deletions)?;
            kg_guard.save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: "Observations deleted successfully".to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "delete_relations".to_string(),
        description: Some("Delete multiple relations from the graph".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "relations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "from": {"type": "string"},
                            "to": {"type": "string"},
                            "relationType": {"type": "string"}
                        },
                        "required": ["from", "to", "relationType"]
                    }
                }
            },
            "required": ["relations"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let relations = args
                .get("relations")
                .ok_or(anyhow::anyhow!("missing arguments `relations`"))?;
            let relations: Vec<Relation> = serde_json::from_value(relations.clone())?;
            let mut kg_guard = kg_clone.lock().unwrap();
            kg_guard.delete_relations(relations)?;
            kg_guard.save_to_file(memory_file_path)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: "Relations deleted successfully".to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "read_graph".to_string(),
        description: Some("Read the entire knowledge graph".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |_req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(*kg_clone.lock().unwrap()).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "search_nodes".to_string(),
        description: Some("Search for nodes in the knowledge graph".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            },
            "required": ["query"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let query = args
                .get("query")
                .ok_or(anyhow::anyhow!("missing argument `query`"))?
                .as_str()
                .ok_or(anyhow::anyhow!("query must be a string"))?;
            let results = kg_clone.lock().unwrap().search_nodes(query)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(results).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    let description = Tool {
        name: "open_nodes".to_string(),
        description: Some("Open specific nodes by their names".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "names": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["names"]
        }),
        output_schema: None,
    };
    let kg_clone = kg.clone();
    server.register_tool(description, move |req: CallToolRequest| {
        let kg_clone = kg_clone.clone();
        Box::pin(async move {
            let args = req.arguments.unwrap_or_default();
            let names = args
                .get("names")
                .ok_or(anyhow::anyhow!("missing arguments `names`"))?;
            let names: Vec<String> = serde_json::from_value(names.clone())?;
            let results = kg_clone.lock().unwrap().open_nodes(names)?;
            Ok(CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: json!(results).to_string(),
                }],
                is_error: None,
                meta: None,
            })
        })
    });

    Ok(())
}
