//! 拼音转换工具。
//!
//! 该模块用于生成搜索、排序或助记字段中常用的全拼和简拼。转换规则保持简单稳定：
//! 能识别的汉字转为无声调小写拼音，非汉字字符按原文保留，避免破坏英文、数字和符号。

use pinyin::ToPinyin;
use serde::Serialize;

/// 拼音转换结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinyinText {
    /// 全拼结果，例如 `中文abc12` 转为 `zhongwenabc12`。
    pub full: String,
    /// 简拼结果，例如 `中文abc12` 转为 `zwabc12`。
    pub simple: String,
}

/// 将输入文本同时转换为全拼和简拼。
///
/// 多音字首版使用 `pinyin` 库提供的默认读音；如果后续业务需要按词组纠音，应在业务层
/// 引入词库或人工别名字段，而不是在该无业务工具中硬编码领域读音。
pub fn to_pinyin_text(input: impl AsRef<str>) -> PinyinText {
    let input = input.as_ref();
    let mut full = String::with_capacity(input.len());
    let mut simple = String::with_capacity(input.len());

    for character in input.chars() {
        match character.to_pinyin() {
            Some(pinyin) => {
                full.push_str(pinyin.plain());
                simple.push_str(pinyin.first_letter());
            }
            None => {
                full.push(character);
                simple.push(character);
            }
        }
    }

    PinyinText { full, simple }
}

/// 只获取全拼结果。
pub fn full_pinyin(input: impl AsRef<str>) -> String {
    to_pinyin_text(input).full
}

/// 只获取简拼结果。
pub fn simple_pinyin(input: impl AsRef<str>) -> String {
    to_pinyin_text(input).simple
}

#[cfg(test)]
mod tests {
    use super::{full_pinyin, simple_pinyin, to_pinyin_text};

    #[test]
    fn pinyin_text_keeps_non_chinese_characters() {
        // 非汉字字符必须按原文保留，满足编码、英文名和数字混排字段的搜索需求。
        let result = to_pinyin_text("中文abc12");

        assert_eq!(result.full, "zhongwenabc12");
        assert_eq!(result.simple, "zwabc12");
    }

    #[test]
    fn pinyin_helpers_return_requested_style() {
        // 便捷函数只返回调用方需要的单一结果，避免业务代码重复拆结构。
        assert_eq!(full_pinyin("拼音-A1"), "pinyin-A1");
        assert_eq!(simple_pinyin("拼音-A1"), "py-A1");
    }

    #[test]
    fn unknown_or_non_hanzi_unicode_is_preserved() {
        // 无拼音映射的字符不能被丢弃，避免用户录入的符号或少数文字丢失。
        let result = to_pinyin_text("𠮷🙂");

        assert_eq!(result.full, "𠮷🙂");
        assert_eq!(result.simple, "𠮷🙂");
    }
}
