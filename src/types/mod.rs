#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }
}
