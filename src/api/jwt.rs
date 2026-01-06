use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub id: String,
    pub exp: i64,
    pub iss: String,
}

/// 解析 JWT Token (仅解析 Payload 部分，不验证签名)
/// 参考 Dart 版本 parseJwt 实现
pub fn parse_jwt(token: &str) -> Result<Claims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".to_string());
    }

    let payload_b64 = parts[1];

    // Base64Url 解码
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    let payload_str =
        String::from_utf8(payload_bytes).map_err(|e| format!("Invalid UTF-8 in payload: {}", e))?;

    let claims: Claims = serde_json::from_str(&payload_str)
        .map_err(|e| format!("Failed to parse JSON claims: {}", e))?;

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jwt() {
        // Example token payload: {"id":"test","exp":12345678,"iss":"test"}
        // b64url: eyJpZCI6InRlc3QiLCJleHAiOjEyMzQ1Njc4LCJpc3MiOiJ0ZXN0In0
        let token = "header.eyJpZCI6InRlc3QiLCJleHAiOjEyMzQ1Njc4LCJpc3MiOiJ0ZXN0In0.signature";
        let claims = parse_jwt(token).unwrap();
        assert_eq!(claims.id, "test");
        assert_eq!(claims.exp, 12345678);
        assert_eq!(claims.iss, "test");
    }
}
