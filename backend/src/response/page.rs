//! 分页参数和分页结果。
//!
//! 分页参数在进入 repository 前完成边界校验，避免超大分页请求拖垮数据库或主线程序列化。

use serde::{Deserialize, Serialize};

use crate::error::app_error::{AppError, AppResult};

pub const DEFAULT_PAGE_NO: u64 = 1;
/// 默认分页大小，兼顾初始加载性能和用户可见数据量。
pub const DEFAULT_PAGE_SIZE: u64 = 20;
/// 服务端分页上限，避免客户端请求超大列表拖垮数据库和序列化。
pub const MAX_PAGE_SIZE: u64 = 100;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageQuery {
    /// 当前页码，从 1 开始，缺省时使用 `DEFAULT_PAGE_NO`。
    pub page_no: Option<u64>,
    /// 每页数量，缺省时使用 `DEFAULT_PAGE_SIZE`，并受 `MAX_PAGE_SIZE` 限制。
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedPageQuery {
    /// 归一化后的页码，保证不小于 1。
    pub page_no: u64,
    /// 归一化后的每页数量，保证处于服务端允许范围内。
    pub page_size: u64,
    /// 数据库查询偏移量，由页码和分页大小计算得出。
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T>
where
    T: Serialize,
{
    /// 当前页数据列表。
    pub records: Vec<T>,
    /// 当前页码。
    pub page_no: u64,
    /// 当前页大小。
    pub page_size: u64,
    /// 满足筛选条件的数据总数。
    pub total: u64,
    /// 总页数，`total = 0` 时固定为 0。
    pub total_pages: u64,
    /// 是否存在下一页，前端可用它控制继续加载按钮。
    pub has_next: bool,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self {
            page_no: Some(DEFAULT_PAGE_NO),
            page_size: Some(DEFAULT_PAGE_SIZE),
        }
    }
}

impl PageQuery {
    /// 校验并归一化分页参数。
    ///
    /// 该方法必须在进入 repository 前调用，避免无效分页参数扩散到 SQL 层。
    pub fn validate_and_normalize(&self) -> AppResult<NormalizedPageQuery> {
        let page_no = self.page_no.unwrap_or(DEFAULT_PAGE_NO);
        let page_size = self.page_size.unwrap_or(DEFAULT_PAGE_SIZE);

        if page_no == 0 {
            return Err(AppError::param_invalid("pageNo 最小值为 1"));
        }
        if page_size == 0 {
            return Err(AppError::param_invalid("pageSize 最小值为 1"));
        }
        if page_size > MAX_PAGE_SIZE {
            return Err(AppError::param_invalid(format!(
                "pageSize 最大值为 {MAX_PAGE_SIZE}"
            )));
        }

        let offset = (page_no - 1)
            .checked_mul(page_size)
            .ok_or_else(|| AppError::param_invalid("pageNo 与 pageSize 计算 offset 溢出"))?;

        Ok(NormalizedPageQuery {
            page_no,
            page_size,
            offset,
        })
    }
}

impl<T> PageResult<T>
where
    T: Serialize,
{
    /// 根据记录、分页参数和总数构造稳定分页响应。
    ///
    /// `total_pages` 使用向上取整，避免最后一页判断出现 off-by-one 错误。
    pub fn new(records: Vec<T>, page: NormalizedPageQuery, total: u64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page.page_size)
        };
        Self {
            records,
            page_no: page.page_no,
            page_size: page.page_size,
            total,
            total_pages,
            has_next: page.page_no < total_pages,
        }
    }
}
