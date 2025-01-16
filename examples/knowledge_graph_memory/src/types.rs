use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub observations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeGraph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

impl KnowledgeGraph {
    pub fn load_from_file(memory_file_path: &str) -> Result<Self> {
        if !Path::new(memory_file_path).exists() {
            return Ok(Self {
                entities: vec![],
                relations: vec![],
            });
        }

        let file = File::open(memory_file_path)?;
        let reader = BufReader::new(file);
        let mut kg = KnowledgeGraph {
            entities: vec![],
            relations: vec![],
        };

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }
            let json_val: serde_json::Value = serde_json::from_str(&line)?;
            if let Some(t) = json_val.get("type").and_then(|v| v.as_str()) {
                match t {
                    "entity" => {
                        let entity: Entity = serde_json::from_value(json_val)?;
                        kg.entities.push(entity);
                    }
                    "relation" => {
                        let relation: Relation = serde_json::from_value(json_val)?;
                        kg.relations.push(relation);
                    }
                    _ => {}
                }
            }
        }

        Ok(kg)
    }

    pub fn save_to_file(&self, memory_file_path: &str) -> Result<()> {
        let mut file = File::create(memory_file_path)?;
        for entity in &self.entities {
            let mut map = serde_json::to_value(entity)?;
            if let Some(obj) = map.as_object_mut() {
                obj.insert(
                    "type".to_string(),
                    serde_json::Value::String("entity".into()),
                );
            }
            let line = serde_json::to_string(&map)?;
            writeln!(file, "{}", line)?;
        }
        for relation in &self.relations {
            let mut map = serde_json::to_value(relation)?;
            if let Some(obj) = map.as_object_mut() {
                obj.insert(
                    "type".to_string(),
                    serde_json::Value::String("relation".into()),
                );
            }
            let line = serde_json::to_string(&map)?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    pub fn create_entities(&mut self, entities: Vec<Entity>) -> Result<Vec<Entity>> {
        let mut newly_added = Vec::new();
        for e in entities {
            if !self.entities.iter().any(|x| x.name == e.name) {
                self.entities.push(e.clone());
                newly_added.push(e);
            }
        }
        Ok(newly_added)
    }

    pub fn create_relations(&mut self, relations: Vec<Relation>) -> Result<Vec<Relation>> {
        let mut newly_added = Vec::new();
        for r in relations {
            if !self.relations.iter().any(|rel| {
                rel.from == r.from && rel.to == r.to && rel.relation_type == r.relation_type
            }) {
                self.relations.push(r.clone());
                newly_added.push(r);
            }
        }
        Ok(newly_added)
    }

    pub fn add_observations(
        &mut self,
        observations: Vec<AddObservationParams>,
    ) -> Result<Vec<AddedObservationResult>> {
        let mut results = Vec::new();

        for obs in observations {
            let entity = self.entities.iter_mut().find(|e| e.name == obs.entity_name);

            if let Some(e) = entity {
                let mut added_contents = Vec::new();
                for content in obs.contents {
                    if !e.observations.contains(&content) {
                        e.observations.push(content.clone());
                        added_contents.push(content);
                    }
                }
                results.push(AddedObservationResult {
                    entity_name: obs.entity_name,
                    added_observations: added_contents,
                });
            } else {
                anyhow::bail!("Entity with name {} not found", obs.entity_name);
            }
        }

        Ok(results)
    }

    pub fn delete_entities(&mut self, entity_names: Vec<String>) -> Result<()> {
        self.entities.retain(|e| !entity_names.contains(&e.name));
        self.relations
            .retain(|r| !entity_names.contains(&r.from) && !entity_names.contains(&r.to));
        Ok(())
    }

    pub fn delete_observations(&mut self, deletions: Vec<DeleteObservationParams>) -> Result<()> {
        for d in deletions {
            if let Some(ent) = self.entities.iter_mut().find(|e| e.name == d.entity_name) {
                ent.observations.retain(|obs| !d.observations.contains(obs));
            }
        }
        Ok(())
    }

    pub fn delete_relations(&mut self, relations: Vec<Relation>) -> Result<()> {
        self.relations.retain(|r| {
            !relations.iter().any(|del| {
                del.from == r.from && del.to == r.to && del.relation_type == r.relation_type
            })
        });
        Ok(())
    }

    pub fn search_nodes(&self, query: &str) -> Result<KnowledgeGraph> {
        let q_lower = query.to_lowercase();

        let filtered_entities: Vec<Entity> = self
            .entities
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q_lower)
                    || e.entity_type.to_lowercase().contains(&q_lower)
                    || e.observations
                        .iter()
                        .any(|obs| obs.to_lowercase().contains(&q_lower))
            })
            .cloned()
            .collect();

        let filtered_entity_names: Vec<String> =
            filtered_entities.iter().map(|e| e.name.clone()).collect();

        let filtered_relations: Vec<Relation> = self
            .relations
            .iter()
            .filter(|r| {
                filtered_entity_names.contains(&r.from) && filtered_entity_names.contains(&r.to)
            })
            .cloned()
            .collect();

        Ok(KnowledgeGraph {
            entities: filtered_entities,
            relations: filtered_relations,
        })
    }

    pub fn open_nodes(&self, names: Vec<String>) -> Result<KnowledgeGraph> {
        let filtered_entities: Vec<Entity> = self
            .entities
            .iter()
            .filter(|e| names.contains(&e.name))
            .cloned()
            .collect();

        let filtered_entity_names: Vec<String> =
            filtered_entities.iter().map(|e| e.name.clone()).collect();

        let filtered_relations: Vec<Relation> = self
            .relations
            .iter()
            .filter(|r| {
                filtered_entity_names.contains(&r.from) && filtered_entity_names.contains(&r.to)
            })
            .cloned()
            .collect();

        Ok(KnowledgeGraph {
            entities: filtered_entities,
            relations: filtered_relations,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct AddObservationParams {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AddedObservationResult {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    #[serde(rename = "addedObservations")]
    pub added_observations: Vec<String>,
}

// For delete_observations
#[derive(Debug, Deserialize)]
pub struct DeleteObservationParams {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub observations: Vec<String>,
}
