//! Validador do subconjunto de JSON Schema usado em `character_schema`.
//!
//! Suportado: raiz `type: object` com `properties`, `required` e
//! `additionalProperties: false`; por campo `type` (`string`, `integer`, `number`,
//! `boolean`), `enum`, `minimum`/`maximum`, `minLength`/`maxLength`.
//! `title`, `description` e `x-enumLabels` são só para a UI.

use domain::DomainError;
use serde_json::{Map, Value};

const FIELD_KEYWORDS: &[&str] = &[
    "type",
    "title",
    "description",
    "enum",
    "x-enumLabels",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
];
const ROOT_KEYWORDS: &[&str] = &["type", "properties", "required", "additionalProperties"];
const FIELD_TYPES: &[&str] = &["string", "integer", "number", "boolean"];

/// Garante que o schema do game pack só usa o que o validador (e o formulário) entendem.
pub fn check_schema(schema: &Value) -> Result<(), String> {
    let root = schema.as_object().ok_or("schema deve ser um objeto")?;
    if let Some(key) = root.keys().find(|k| !ROOT_KEYWORDS.contains(&k.as_str())) {
        return Err(format!("palavra-chave não suportada na raiz: {key}"));
    }
    if root.get("type") != Some(&Value::from("object")) {
        return Err("raiz precisa de `type: object`".to_owned());
    }
    if root
        .get("additionalProperties")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("`additionalProperties` deve ser booleano".to_owned());
    }

    let empty = Map::new();
    let properties = match root.get("properties") {
        Some(v) => v.as_object().ok_or("`properties` deve ser um objeto")?,
        None => &empty,
    };
    for (name, field) in properties {
        check_field(name, field)?;
    }

    for required in required_list(root)? {
        if !properties.contains_key(required) {
            return Err(format!("`required` cita campo inexistente: {required}"));
        }
    }
    Ok(())
}

fn check_field(name: &str, field: &Value) -> Result<(), String> {
    let field = field
        .as_object()
        .ok_or_else(|| format!("{name}: definição deve ser um objeto"))?;
    if let Some(key) = field.keys().find(|k| !FIELD_KEYWORDS.contains(&k.as_str())) {
        return Err(format!("{name}: palavra-chave não suportada: {key}"));
    }
    let kind = field
        .get("type")
        .and_then(Value::as_str)
        .filter(|t| FIELD_TYPES.contains(t))
        .ok_or_else(|| format!("{name}: `type` deve ser um de {FIELD_TYPES:?}"))?;

    if let Some(options) = field.get("enum") {
        let options = options
            .as_array()
            .filter(|o| !o.is_empty())
            .ok_or_else(|| format!("{name}: `enum` deve ser uma lista não vazia"))?;
        if let Some(bad) = options.iter().find(|o| type_error(kind, o).is_some()) {
            return Err(format!("{name}: valor de `enum` fora do tipo: {bad}"));
        }
        if let Some(labels) = field.get("x-enumLabels") {
            if labels.as_array().map(Vec::len) != Some(options.len()) {
                return Err(format!(
                    "{name}: `x-enumLabels` deve ter o tamanho de `enum`"
                ));
            }
        }
    }
    for key in ["minimum", "maximum"] {
        if field.get(key).is_some_and(|v| !v.is_number()) {
            return Err(format!("{name}: `{key}` deve ser número"));
        }
    }
    for key in ["minLength", "maxLength"] {
        if field.get(key).is_some_and(|v| v.as_u64().is_none()) {
            return Err(format!("{name}: `{key}` deve ser inteiro não negativo"));
        }
    }
    Ok(())
}

/// Valida `attributes` de um personagem. O erro aponta o campo como `attributes.<nome>`.
pub fn validate(schema: &Value, attrs: &Value) -> Result<(), DomainError> {
    let attrs = attrs
        .as_object()
        .ok_or_else(|| invalid("attributes", "deve ser um objeto"))?;
    let root = schema
        .as_object()
        .ok_or_else(|| invalid("attributes", "schema inválido"))?;
    let empty = Map::new();
    let properties = root
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let closed = root.get("additionalProperties") == Some(&Value::Bool(false));

    for (name, value) in attrs {
        match properties.get(name) {
            Some(field) if !value.is_null() => validate_field(name, field, value)?,
            Some(_) => {}
            None if closed => return Err(invalid_attr(name, "campo desconhecido")),
            None => {}
        }
    }

    let required = required_list(root).map_err(|msg| invalid("attributes", &msg))?;
    if let Some(missing) = required
        .into_iter()
        .find(|name| attrs.get(*name).is_none_or(Value::is_null))
    {
        return Err(invalid_attr(missing, "obrigatório"));
    }
    Ok(())
}

fn validate_field(name: &str, field: &Value, value: &Value) -> Result<(), DomainError> {
    let kind = field
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if let Some(message) = type_error(kind, value) {
        return Err(invalid_attr(name, message));
    }

    if let Some(options) = field.get("enum").and_then(Value::as_array) {
        if !options.contains(value) {
            return Err(invalid_attr(name, "valor fora das opções"));
        }
    }

    if let Some(number) = value.as_f64() {
        if let Some(min) = field.get("minimum").and_then(Value::as_f64) {
            if number < min {
                return Err(invalid_attr(name, &format!("mínimo {min}")));
            }
        }
        if let Some(max) = field.get("maximum").and_then(Value::as_f64) {
            if number > max {
                return Err(invalid_attr(name, &format!("máximo {max}")));
            }
        }
    }

    if let Some(text) = value.as_str() {
        let len = text.chars().count() as u64;
        if let Some(min) = field.get("minLength").and_then(Value::as_u64) {
            if len < min {
                return Err(invalid_attr(name, &format!("mínimo de {min} caracteres")));
            }
        }
        if let Some(max) = field.get("maxLength").and_then(Value::as_u64) {
            if len > max {
                return Err(invalid_attr(name, &format!("máximo de {max} caracteres")));
            }
        }
    }
    Ok(())
}

fn type_error(kind: &str, value: &Value) -> Option<&'static str> {
    let ok = match kind {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => false,
    };
    (!ok).then_some(match kind {
        "string" => "deve ser texto",
        "integer" => "deve ser número inteiro",
        "number" => "deve ser número",
        "boolean" => "deve ser verdadeiro/falso",
        _ => "tipo não suportado",
    })
}

fn required_list(root: &Map<String, Value>) -> Result<Vec<&str>, String> {
    match root.get("required") {
        None => Ok(Vec::new()),
        Some(list) => list
            .as_array()
            .ok_or("`required` deve ser uma lista")?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| "`required` deve conter textos".to_owned())
            })
            .collect(),
    }
}

fn invalid_attr(name: &str, message: &str) -> DomainError {
    invalid(&format!("attributes.{name}"), message)
}

fn invalid(field: &str, message: &str) -> DomainError {
    DomainError::Validation {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    fn schema() -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "cultivo": { "type": "string", "enum": ["sabio", "demonio"], "x-enumLabels": ["Sábio", "Demônio"] },
                "reino": { "type": "integer", "minimum": 0, "maximum": 10 },
                "apelido": { "type": "string", "maxLength": 3 },
                "pvp": { "type": "boolean" }
            },
            "required": ["reino"]
        })
    }

    fn field_of(err: DomainError) -> String {
        match err {
            DomainError::Validation { field, .. } => field,
            other => panic!("esperava Validation, veio {other:?}"),
        }
    }

    #[test]
    fn schema_is_accepted() {
        check_schema(&schema()).unwrap();
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        for bad in [
            json!({ "type": "array" }),
            json!({ "type": "object", "properties": { "x": { "type": "object" } } }),
            json!({ "type": "object", "properties": { "x": { "type": "string", "pattern": "a" } } }),
            json!({ "type": "object", "properties": { "x": { "type": "integer", "enum": ["a"] } } }),
            json!({ "type": "object", "properties": {}, "required": ["x"] }),
        ] {
            assert!(check_schema(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn valid_attributes_pass() {
        validate(
            &schema(),
            &json!({ "reino": 3, "cultivo": "sabio", "pvp": true, "apelido": null }),
        )
        .unwrap();
    }

    #[test]
    fn errors_point_to_the_field() {
        let cases = [
            (json!({}), "attributes.reino"),
            (json!({ "reino": 11 }), "attributes.reino"),
            (json!({ "reino": 1.5 }), "attributes.reino"),
            (json!({ "reino": "3" }), "attributes.reino"),
            (
                json!({ "reino": 1, "cultivo": "outro" }),
                "attributes.cultivo",
            ),
            (
                json!({ "reino": 1, "apelido": "longo" }),
                "attributes.apelido",
            ),
            (json!({ "reino": 1, "extra": 1 }), "attributes.extra"),
            (json!([]), "attributes"),
        ];
        for (attrs, field) in cases {
            let err = validate(&schema(), &attrs).unwrap_err();
            assert_eq!(field_of(err), field, "{attrs}");
        }
    }
}
