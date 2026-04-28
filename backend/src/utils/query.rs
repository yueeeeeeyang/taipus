//! HTTP query 参数工具。
//!
//! 该工具服务中间件、resolver 等只能拿到 `Uri` 的底层逻辑；普通 handler 仍应优先使用
//! Axum `Query<T>` extractor 进行结构化参数解析。

use http::Uri;

/// 从 URI query 中读取并解码指定参数。
pub fn query_param(uri: &Uri, key: &str) -> Option<String> {
    uri.query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(name), Some(value)) if name == key => {
                    let decoded = decode_query_component(value)?;
                    let trimmed = decoded.trim();
                    (!trimmed.is_empty()).then(|| trimmed.to_string())
                }
                _ => None,
            }
        })
    })
}

fn decode_query_component(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let raw = value.as_bytes();
    let mut index = 0;

    while index < raw.len() {
        match raw[index] {
            b'+' => {
                bytes.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < raw.len() => {
                let high = hex_value(raw[index + 1])?;
                let low = hex_value(raw[index + 2])?;
                bytes.push((high << 4) | low);
                index += 3;
            }
            b'%' => return None,
            byte => {
                bytes.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::query_param;

    #[test]
    fn query_param_decodes_percent_encoded_value() {
        // 前端 URLSearchParams 会把 `/` 编码为 `%2F`，后端解析后必须还原。
        let uri: Uri = "/api/v1/i18n/system_resources?timeZone=America%2FNew_York"
            .parse()
            .unwrap();

        assert_eq!(
            query_param(&uri, "timeZone"),
            Some("America/New_York".to_string())
        );
    }

    #[test]
    fn query_param_rejects_invalid_percent_encoding() {
        // 非法百分号编码不能原样进入 locale 或 time zone 校验逻辑。
        let uri: Uri = "/api/v1/i18n/system_resources?timeZone=America%2GNew_York"
            .parse()
            .unwrap();

        assert_eq!(query_param(&uri, "timeZone"), None);
    }
}
