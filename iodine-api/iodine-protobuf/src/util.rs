pub fn prost_struct_to_json_value(prost_struct: prost_types::Struct) -> serde_json::Value {
    serde_json::Value::Object(
        prost_struct
            .fields
            .into_iter()
            .map(|(k, v)| (k, prost_to_json_value(v)))
            .collect(),
    )
}

pub fn json_value_to_prost_struct(json_value: serde_json::Value) -> Option<prost_types::Struct> {
    match json_value {
        serde_json::Value::Object(v) => Some(to_struct(v)),
        _ => None,
    }
}

fn prost_to_json_value(prost_value: prost_types::Value) -> serde_json::Value {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;
    match prost_value.kind {
        Some(x) => match x {
            NullValue(_) => Null,
            BoolValue(v) => Bool(v),
            NumberValue(n) => Number(serde_json::Number::from_f64(n).unwrap()),
            StringValue(s) => String(s),
            ListValue(lst) => Array(lst.values.into_iter().map(prost_to_json_value).collect()),
            StructValue(v) => Object(
                v.fields
                    .into_iter()
                    .map(|(k, v)| (k, prost_to_json_value(v)))
                    .collect(),
            ),
        },
        None => Null,
    }
}

fn json_value_to_prost(json_value: serde_json::Value) -> prost_types::Value {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;
    prost_types::Value {
        kind: Some(match json_value {
            Null => NullValue(0 /* wat? */),
            Bool(v) => BoolValue(v),
            Number(n) => NumberValue(n.as_f64().expect("Non-f64-representable number")),
            String(s) => StringValue(s),
            Array(v) => ListValue(prost_types::ListValue {
                values: v.into_iter().map(json_value_to_prost).collect(),
            }),
            Object(v) => StructValue(to_struct(v)),
        }),
    }
}

fn to_struct(json: serde_json::Map<String, serde_json::Value>) -> prost_types::Struct {
    prost_types::Struct {
        fields: json
            .into_iter()
            .map(|(k, v)| (k, json_value_to_prost(v)))
            .collect(),
    }
}
