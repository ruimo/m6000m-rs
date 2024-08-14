use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jsonl {
  pub raw: es51986::Output,
  pub value: Option<es51986::OutputValue>,
}