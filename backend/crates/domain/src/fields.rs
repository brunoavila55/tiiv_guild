//! Campos dinâmicos definidos por dados (campos customizados de membro e, depois, os schemas
//! de módulos): um subconjunto de JSON Schema e a validação dos valores contra ele.
//!
//! Suportado: `type` (`string`/`integer`/`number`/`boolean`), `title`, `enum`,
//! `minimum`/`maximum`, `minLength`/`maxLength` e `required`. Como o jsonb não preserva a
//! ordem das chaves, a ordem de exibição vai em `x-order`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, FieldErrors};

pub const MAX_FIELDS: usize = 30;
pub const KEY_MAX: usize = 32;
pub const TITLE_MAX: usize = 40;
pub const OPTIONS_MAX: usize = 50;
pub const OPTION_MAX: usize = 60;
/// Limite de qualquer texto, mesmo sem `maxLength`.
pub const STRING_MAX: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
}

/// Um campo (uma entrada de `properties`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub kind: FieldType,
    pub title: String,
    #[serde(rename = "enum", default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "minLength", default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(rename = "maxLength", default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ObjectType {
    #[default]
    Object,
}

/// `{"type": "object", "properties": {…}, "required": […], "x-order": […]}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSchema {
    #[serde(rename = "type", default)]
    kind: ObjectType,
    #[serde(default)]
    pub properties: BTreeMap<String, FieldDef>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "x-order", default)]
    pub order: Vec<String>,
}

impl ObjectSchema {
    /// Lê um schema salvo; um valor que não é schema vira schema vazio.
    pub fn from_stored(value: Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    /// Lê um schema enviado pelo cliente (erro de formato vira erro de validação).
    pub fn parse(value: Value) -> Result<Self, Error> {
        serde_json::from_value(value)
            .map_err(|e| Error::field("schema", format!("schema inválido: {e}")))
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn is_required(&self, key: &str) -> bool {
        self.required.iter().any(|r| r == key)
    }

    /// Campos na ordem de exibição: a de `x-order`, depois os que faltarem, por chave.
    pub fn ordered(&self) -> impl Iterator<Item = (&str, &FieldDef)> {
        let listed = self
            .order
            .iter()
            .filter_map(|k| self.properties.get_key_value(k));
        let rest = self
            .properties
            .iter()
            .filter(|(k, _)| !self.order.contains(k));
        listed.chain(rest).map(|(k, f)| (k.as_str(), f))
    }

    /// Valida o schema e devolve a versão normalizada (títulos e opções sem espaços nas
    /// pontas, `required` e `x-order` só com chaves existentes e sem repetição).
    /// Erros por campo em `properties.<chave>`.
    pub fn normalize(self) -> Result<Self, Error> {
        let mut errors = FieldErrors::new();
        if self.properties.len() > MAX_FIELDS {
            errors.insert("schema".into(), format!("no máximo {MAX_FIELDS} campos"));
        }
        let mut properties = BTreeMap::new();
        for (key, field) in self.properties {
            let path = format!("properties.{key}");
            if let Err(msg) = check_key(&key) {
                errors.insert(path, msg);
                continue;
            }
            match field.normalize() {
                Ok(f) => {
                    properties.insert(key, f);
                }
                Err(msg) => {
                    errors.insert(path, msg);
                }
            }
        }
        if let Some(missing) = self.required.iter().find(|k| {
            !properties.contains_key(*k) && !errors.contains_key(&format!("properties.{k}"))
        }) {
            errors.insert("required".into(), format!("campo inexistente: {missing}"));
        }
        if !errors.is_empty() {
            return Err(Error::Validation(errors));
        }
        let dedup = |keys: Vec<String>| {
            let mut seen = BTreeSet::new();
            keys.into_iter()
                .filter(|k| properties.contains_key(k) && seen.insert(k.clone()))
                .collect::<Vec<_>>()
        };
        let required = dedup(self.required);
        let mut order = dedup(self.order);
        let rest: Vec<_> = properties
            .keys()
            .filter(|k| !order.contains(k))
            .cloned()
            .collect();
        order.extend(rest);
        Ok(Self {
            kind: ObjectType::Object,
            properties,
            required,
            order,
        })
    }

    /// Valida os valores enviados e devolve só os preenchidos, já normalizados.
    /// Vazio (`null` ou texto em branco) conta como ausente. Erros em `<prefix>.<chave>`.
    pub fn validate_values(
        &self,
        prefix: &str,
        input: &Map<String, Value>,
    ) -> Result<Map<String, Value>, Error> {
        let mut errors = FieldErrors::new();
        let mut out = Map::new();
        for key in input.keys() {
            if !self.properties.contains_key(key) {
                errors.insert(format!("{prefix}.{key}"), "campo desconhecido".into());
            }
        }
        for (key, field) in self.ordered() {
            match input.get(key).filter(|v| !is_blank(v)) {
                None if self.is_required(key) => {
                    errors.insert(format!("{prefix}.{key}"), "obrigatório".into());
                }
                None => {}
                Some(v) => match field.check_value(v) {
                    Ok(v) => {
                        out.insert(key.to_owned(), v);
                    }
                    Err(msg) => {
                        errors.insert(format!("{prefix}.{key}"), msg);
                    }
                },
            }
        }
        if errors.is_empty() {
            Ok(out)
        } else {
            Err(Error::Validation(errors))
        }
    }

    /// Valores salvos que ainda têm campo no schema (campos removidos não aparecem).
    pub fn project(&self, stored: &Value) -> Map<String, Value> {
        stored
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, _)| self.properties.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

fn check_key(key: &str) -> Result<(), String> {
    let mut chars = key.chars();
    let ok = key.len() <= KEY_MAX
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "chave inválida: use letras minúsculas, números e _ (até {KEY_MAX}), começando por letra"
        ))
    }
}

fn is_blank(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        _ => false,
    }
}

/// Número inteiro de JSON (aceita `3.0`).
fn as_integer(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| {
        v.as_f64()
            .filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
            .map(|f| f as i64)
    })
}

impl FieldDef {
    fn normalize(mut self) -> Result<Self, String> {
        self.title = self.title.trim().to_owned();
        let title_len = self.title.chars().count();
        if !(1..=TITLE_MAX).contains(&title_len) {
            return Err(format!("o nome deve ter entre 1 e {TITLE_MAX} caracteres"));
        }
        let numeric = matches!(self.kind, FieldType::Integer | FieldType::Number);
        let bounds = self.minimum.is_some() || self.maximum.is_some();
        let lengths = self.min_length.is_some() || self.max_length.is_some();

        if bounds && !numeric {
            return Err("mínimo e máximo só valem para números".into());
        }
        if lengths && self.kind != FieldType::String {
            return Err("tamanho mínimo e máximo só valem para texto".into());
        }
        if let Some(options) = self.options.take() {
            if !(numeric || self.kind == FieldType::String) {
                return Err("opções só valem para texto e números".into());
            }
            if bounds || lengths {
                return Err("use opções ou limites, não os dois".into());
            }
            self.options = Some(self.normalize_options(options)?);
        }
        for bound in [self.minimum, self.maximum].into_iter().flatten() {
            if !bound.is_finite() || (self.kind == FieldType::Integer && bound.fract() != 0.0) {
                return Err("mínimo e máximo devem ser números válidos do tipo do campo".into());
            }
        }
        if let (Some(min), Some(max)) = (self.minimum, self.maximum) {
            if min > max {
                return Err("o mínimo não pode ser maior que o máximo".into());
            }
        }
        if self.max_length.is_some_and(|m| m == 0 || m > STRING_MAX) {
            return Err(format!("o tamanho máximo deve ser entre 1 e {STRING_MAX}"));
        }
        if self.min_length.unwrap_or(0) > self.max_length.unwrap_or(STRING_MAX) {
            return Err("o tamanho mínimo não pode ser maior que o máximo".into());
        }
        Ok(self)
    }

    fn normalize_options(&self, options: Vec<Value>) -> Result<Vec<Value>, String> {
        if options.is_empty() || options.len() > OPTIONS_MAX {
            return Err(format!("informe entre 1 e {OPTIONS_MAX} opções"));
        }
        let mut out: Vec<Value> = Vec::with_capacity(options.len());
        for option in options {
            let option = match self.kind {
                FieldType::String => match option.as_str().map(str::trim) {
                    Some(s) if (1..=OPTION_MAX).contains(&s.chars().count()) => Value::from(s),
                    _ => {
                        return Err(format!(
                            "cada opção deve ter entre 1 e {OPTION_MAX} caracteres"
                        ))
                    }
                },
                FieldType::Integer => match as_integer(&option) {
                    Some(n) => Value::from(n),
                    None => return Err("as opções devem ser números inteiros".into()),
                },
                _ => match option.as_f64() {
                    Some(_) => option,
                    None => return Err("as opções devem ser números".into()),
                },
            };
            if out.iter().any(|o| same_value(o, &option)) {
                return Err("opções repetidas".into());
            }
            out.push(option);
        }
        Ok(out)
    }

    /// Valor normalizado, ou a mensagem de erro.
    fn check_value(&self, v: &Value) -> Result<Value, String> {
        let value = match self.kind {
            FieldType::String => {
                let s = v.as_str().ok_or("deve ser texto")?.trim();
                let len = u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
                let min = self.min_length.unwrap_or(0);
                let max = self.max_length.unwrap_or(STRING_MAX);
                if len < min || len > max {
                    return Err(if min > 0 {
                        format!("deve ter entre {min} e {max} caracteres")
                    } else {
                        format!("máximo de {max} caracteres")
                    });
                }
                Value::from(s)
            }
            FieldType::Integer => Value::from(as_integer(v).ok_or("deve ser um número inteiro")?),
            FieldType::Number => {
                v.as_f64().ok_or("deve ser um número")?;
                v.clone()
            }
            FieldType::Boolean => Value::from(v.as_bool().ok_or("deve ser sim ou não")?),
        };
        if let Some(n) = value.as_f64() {
            if let Some(min) = self.minimum.filter(|m| n < *m) {
                return Err(format!("deve ser no mínimo {min}"));
            }
            if let Some(max) = self.maximum.filter(|m| n > *m) {
                return Err(format!("deve ser no máximo {max}"));
            }
        }
        if let Some(options) = &self.options {
            if !options.iter().any(|o| same_value(o, &value)) {
                return Err("escolha uma das opções".into());
            }
        }
        Ok(value)
    }
}

/// Igualdade que trata `2` e `2.0` como o mesmo número.
fn same_value(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema(v: Value) -> Result<ObjectSchema, Error> {
        ObjectSchema::parse(v)?.normalize()
    }

    fn fields(err: Error) -> FieldErrors {
        match err {
            Error::Validation(f) => f,
            other => panic!("esperava validação, veio {other:?}"),
        }
    }

    fn sample() -> ObjectSchema {
        schema(json!({
            "type": "object",
            "properties": {
                "discord": { "type": "string", "title": " Discord ", "maxLength": 20 },
                "level": { "type": "integer", "title": "Nível", "minimum": 1, "maximum": 100 },
                "role": { "type": "string", "title": "Função", "enum": ["Tank", " Healer "] },
                "mic": { "type": "boolean", "title": "Microfone" },
                "score": { "type": "number", "title": "Nota" }
            },
            "required": ["level", "level", "discord"],
            "x-order": ["role", "nope", "level"]
        }))
        .unwrap()
    }

    #[test]
    fn normalizes_schema() {
        let s = sample();
        assert_eq!(s.properties["discord"].title, "Discord");
        assert_eq!(
            s.properties["role"].options,
            Some(vec![json!("Tank"), json!("Healer")])
        );
        assert_eq!(s.required, vec!["level", "discord"]);
        let order: Vec<_> = s.ordered().map(|(k, _)| k).collect();
        assert_eq!(order, vec!["role", "level", "discord", "mic", "score"]);
        assert_eq!(s.order, order);
        // ida e volta pelo jsonb
        assert_eq!(ObjectSchema::from_stored(s.to_value()), s);
    }

    #[test]
    fn rejects_bad_schemas() {
        let bad = |props: Value| fields(schema(json!({ "properties": props })).unwrap_err());
        let e = bad(json!({ "Bad-Key": { "type": "string", "title": "x" } }));
        assert!(e.contains_key("properties.Bad-Key"));
        let e = bad(json!({ "a": { "type": "string", "title": "  " } }));
        assert!(e.contains_key("properties.a"));
        let e = bad(json!({ "a": { "type": "boolean", "title": "A", "enum": [true] } }));
        assert!(e.contains_key("properties.a"));
        let e = bad(json!({ "a": { "type": "string", "title": "A", "minimum": 1 } }));
        assert!(e.contains_key("properties.a"));
        let e = bad(json!({ "a": { "type": "integer", "title": "A", "minimum": 1.5 } }));
        assert!(e.contains_key("properties.a"));
        let e =
            bad(json!({ "a": { "type": "integer", "title": "A", "minimum": 5, "maximum": 1 } }));
        assert!(e.contains_key("properties.a"));
        let e = bad(json!({ "a": { "type": "string", "title": "A", "enum": ["x", " x"] } }));
        assert!(e.contains_key("properties.a"));
        let e = bad(json!({ "a": { "type": "string", "title": "A", "maxLength": 9999 } }));
        assert!(e.contains_key("properties.a"));

        assert!(
            schema(json!({ "properties": { "a": { "type": "date", "title": "A" } } })).is_err()
        );
        assert!(schema(
            json!({ "properties": { "a": { "type": "string", "title": "A", "pattern": "x" } } })
        )
        .is_err());
        let e = fields(schema(json!({ "properties": {}, "required": ["ghost"] })).unwrap_err());
        assert!(e.contains_key("required"));
        let many: Map<String, Value> = (0..=MAX_FIELDS)
            .map(|i| (format!("f{i}"), json!({ "type": "boolean", "title": "x" })))
            .collect();
        assert!(fields(schema(json!({ "properties": many })).unwrap_err()).contains_key("schema"));
    }

    #[test]
    fn validates_values() {
        let s = sample();
        let input = json!({
            "discord": "  fulano ", "level": 10.0, "role": "Healer", "mic": false, "score": " "
        });
        let out = s
            .validate_values("fields", input.as_object().unwrap())
            .unwrap();
        assert_eq!(
            Value::Object(out),
            json!({ "discord": "fulano", "level": 10, "role": "Healer", "mic": false })
        );

        let input = json!({
            "discord": "x".repeat(21), "level": 0, "role": "DPS", "mic": "sim", "score": "1", "ghost": 1
        });
        let e = fields(
            s.validate_values("fields", input.as_object().unwrap())
                .unwrap_err(),
        );
        for k in ["discord", "level", "role", "mic", "score", "ghost"] {
            assert!(e.contains_key(&format!("fields.{k}")), "{k}: {e:?}");
        }

        let e = fields(s.validate_values("fields", &Map::new()).unwrap_err());
        assert_eq!(e.len(), 2, "{e:?}");
        assert_eq!(e["fields.level"], "obrigatório");
        let e = fields(
            s.validate_values("fields", json!({ "level": 1.5 }).as_object().unwrap())
                .unwrap_err(),
        );
        assert!(e.contains_key("fields.level"));
    }

    #[test]
    fn projects_stored_values() {
        let s = sample();
        let stored = json!({ "level": 3, "old_field": "x" });
        assert_eq!(Value::Object(s.project(&stored)), json!({ "level": 3 }));
        assert!(s.project(&Value::Null).is_empty());
        assert_eq!(
            ObjectSchema::from_stored(json!("lixo")),
            ObjectSchema::default()
        );
    }
}
