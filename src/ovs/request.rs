use crate::{ovs::Object, ovs::Ovs};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    ListDbs,
    GetSchema,
    Transact,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    method: Method,
    #[serde(rename = "id")]
    msg_id: i32,
    params: Vec<Param>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Param {
    Db(String),
    Op(Op),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Op {
    op: String,
    table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#where: Option<Vec<OpCondition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    row: Option<Row>,
    #[serde(rename = "uuid-name", skip_serializing_if = "Option::is_none")]
    uuid_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mutations: Option<Vec<(String, String, (String, String))>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum OpCondition {
    Nested((String, String, (String, String))),
    Flat((String, String, String)),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Row {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    interfaces: Option<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
}

impl Request {
    pub fn new(method: Method, ovs: &Ovs) -> Self {
        let request_params = match method {
            Method::Transact => vec![Param::Db(ovs.db.clone())],
            _ => vec![],
        };

        Self {
            method: method,
            msg_id: ovs.msg_id,
            params: request_params,
        }
    }

    pub fn select(mut self, table: String) -> Self {
        self.params.push(Param::Op(Op {
            op: String::from("select"),
            table: table,
            r#where: Some(vec![]),
            ..Default::default()
        }));
        self
    }

    pub fn insert<T: Object>(mut self, obj: &T) -> Self {
        match obj.get_table().as_str() {
            "Bridge" => {
                self.params.push(Param::Op(Op {
                    op: String::from("insert"),
                    table: obj.get_table(),
                    row: Some(Row {
                        name: obj.get_name(),
                        ..Default::default()
                    }),
                    uuid_name: Some(String::from("new_bridge")),
                    ..Default::default()
                }));

                self.params.push(Param::Op(Op {
                    op: String::from("mutate"),
                    table: String::from("Open_vSwitch"),
                    r#where: Some(vec![]),
                    mutations: Some(vec![(
                        String::from("bridges"),
                        String::from("insert"),
                        (String::from("named-uuid"), String::from("new_bridge")),
                    )]),
                    ..Default::default()
                }));
            }
            "Port" => {
                self.params.push(Param::Op(Op {
                    op: String::from("insert"),
                    table: obj.get_table(),
                    row: Some(Row {
                        name: obj.get_name(),
                        interfaces: Some((
                            String::from("named-uuid"),
                            String::from("new_interface"),
                        )),
                        ..Default::default()
                    }),
                    uuid_name: Some(String::from("new_port")),
                    ..Default::default()
                }));

                self.params.push(Param::Op(Op {
                    op: String::from("insert"),
                    table: String::from("Interface"),
                    row: Some(Row {
                        name: obj.get_name(),
                        r#type: Some(String::from("internal")),
                        ..Default::default()
                    }),
                    uuid_name: Some(String::from("new_interface")),
                    ..Default::default()
                }));


                self.params.push(Param::Op(Op {
                    op: String::from("mutate"),
                    table: String::from("Bridge"),
                    r#where: Some(vec![OpCondition::Flat((
                        String::from("name"),
                        String::from("=="),
                        String::from("virtus-int"),
                    ))]),
                    mutations: Some(vec![(
                        String::from("ports"),
                        String::from("insert"),
                        (String::from("named-uuid"), String::from("new_port")),
                    )]),
                    ..Default::default()
                }));
            }
            _ => {
                self.params.push(Param::Op(Op {
                    op: String::from("insert"),
                    table: obj.get_table(),
                    row: Some(Row {
                        name: obj.get_name(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        };

        self
    }

    pub fn delete<T: Object>(mut self, obj: &T) -> Self {
        match obj.get_table().as_str() {
            "Bridge" => {
                self.params.push(Param::Op(Op {
                    op: String::from("delete"),
                    table: obj.get_table(),
                    r#where: Some(vec![OpCondition::Nested((
                        String::from("_uuid"),
                        String::from("=="),
                        (String::from("uuid"), obj.get_uuid().unwrap()),
                    ))]),
                    ..Default::default()
                }));

                self.params.push(Param::Op(Op {
                    op: String::from("mutate"),
                    table: String::from("Open_vSwitch"),
                    r#where: Some(vec![]),
                    mutations: Some(vec![(
                        String::from("bridges"),
                        String::from("delete"),
                        (String::from("uuid"), obj.get_uuid().unwrap()),
                    )]),
                    ..Default::default()
                }));
            }
            "Port" => {
                self.params.push(Param::Op(Op {
                    op: String::from("mutate"),
                    table: String::from("Bridge"),
                    r#where: Some(vec![OpCondition::Flat((
                        String::from("name"),
                        String::from("=="),
                        String::from("virtus-int"),
                    ))]),
                    mutations: Some(vec![(
                        String::from("ports"),
                        String::from("delete"),
                        (String::from("uuid"), obj.get_uuid().unwrap()),
                    )]),
                    ..Default::default()
                }));

                self.params.push(Param::Op(Op {
                    op: String::from("delete"),
                    table: String::from("Interface"),
                    r#where: Some(vec![OpCondition::Flat((
                        String::from("name"),
                        String::from("=="),
                        obj.get_name(),
                    ))]),
                    ..Default::default()
                }));
 
                self.params.push(Param::Op(Op {
                    op: String::from("delete"),
                    table: obj.get_table(),
                    r#where: Some(vec![OpCondition::Nested((
                        String::from("_uuid"),
                        String::from("=="),
                        (String::from("uuid"), obj.get_uuid().unwrap()),
                    ))]),
                    ..Default::default()
                }));
           }
            _ => {
                self.params.push(Param::Op(Op {
                    op: String::from("delete"),
                    table: obj.get_table(),
                    r#where: Some(vec![OpCondition::Nested((
                        String::from("_uuid"),
                        String::from("=="),
                        (String::from("uuid"), obj.get_uuid().unwrap()),
                    ))]),
                    ..Default::default()
                }));
            }
        }

        self
    }
}
