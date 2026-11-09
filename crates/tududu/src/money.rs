//! 金额相关的转换。
//!
//! 库里用 NUMERIC 定点存储，对外接口按普通小数返回，前端不用关心实现细节。

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::Serializer;

/// 序列化成 JSON 数字，前端直接当 number 用。
pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(value.to_f64().unwrap_or_default())
}

/// f64 转定点，保留四位小数。
pub fn from_f64(value: f64) -> Decimal {
    Decimal::from_f64_retain(value)
        .unwrap_or_default()
        .round_dp(4)
}

/// 字符串或数字都能解析，方便直接收表单值。
pub fn parse_decimal(raw: &str) -> Option<Decimal> {
    raw.trim().parse::<Decimal>().ok().map(|value| value.round_dp(4))
}

/// 反序列化：数字或字符串都接受，统一成定点。
pub fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(f64),
        Text(String),
    }

    let value = Raw::deserialize(deserializer)?;
    let parsed = match value {
        Raw::Number(number) => {
            if !number.is_finite() {
                return Err(serde::de::Error::custom("金额不合法"));
            }
            from_f64(number)
        }
        Raw::Text(text) => parse_decimal(&text)
            .ok_or_else(|| serde::de::Error::custom("金额不合法"))?,
    };

    Ok(parsed)
}
