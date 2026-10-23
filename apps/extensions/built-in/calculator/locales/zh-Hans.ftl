extension-name = 计算器

action-copy-result = 复制结果

error-unsupported-protocol = 计算器不支持请求的扩展协议。
error-not-initialized = 计算器必须先完成初始化才能处理请求。
error-cancelled = 计算已取消。
error-unknown-action = 此计算结果或操作已不可用，请重新计算表达式。
error-unsupported-message = 计算器收到了不支持的请求。

error-host-service = { $outcome ->
    [cancelled] 复制计算结果的请求已在受理前取消。
    [rejected] 宿主未受理复制计算结果的请求，请查看详情后重试。
   *[failed] 复制计算结果时报告了失败，请查看详情后再决定是否重试。
    [unconfirmed] Nanika 无法确认计算结果是否已复制，请先检查剪贴板内容，再决定是否重试。
    }
