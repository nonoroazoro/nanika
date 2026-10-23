extension-name = 剪贴板历史
error-text-read = 无法加载剪贴板文本，请重试或重新打开剪贴板历史。
setting-clipboard-syntax-highlighting-title = 语法高亮
setting-clipboard-syntax-highlighting-description = 在文本预览中高亮显示识别出的代码和 Markdown。
group-appearance = 外观
setting-clipboard-max-entries-title = 最多保留条目
setting-clipboard-max-entries-description = 本地保留的剪贴板历史条目上限。
group-history-retention = 历史保留
setting-clipboard-max-age-days-title = 最长保留天数
setting-clipboard-max-age-days-description = 本地剪贴板历史的最长保留天数。
group-danger-zone = 危险操作
setting-clipboard-clear-description = 永久删除所有剪贴板历史。此操作无法撤销。
setting-clipboard-clear-button-label = 清除

action-copy = 复制到剪贴板
action-clipboard-clear-title = 清除历史记录
action-clipboard-clear-confirmation-title = 清除全部历史记录？
action-clipboard-clear-confirmation-description = 全部剪贴板历史记录将被永久删除。此操作无法撤销。

view-history-description = 搜索和复制之前的剪贴板内容。
category-clipboard = 剪贴板
keyword-clipboard = 剪贴板
keyword-history = 历史
keyword-paste = 粘贴

view-search-placeholder = 搜索剪贴板历史
view-empty-title = 暂无剪贴板历史
view-empty-description = 复制的内容将显示在这里。
view-empty-filtered-title = 没有匹配的记录
view-empty-filtered-description = 请尝试其他搜索词或筛选条件。
filter-all = 所有类型
content-text = 文本
content-files = 文件
content-images = 图片
content-image = 图片
metadata-content-type = 内容类型
metadata-size = 大小

entry-image = 图片 { $width } × { $height }
entry-files = { $name }和另外 { $remaining } 个文件

error-clear-history = 无法清空剪贴板历史，请查看详细信息后再决定是否重试。
error-action-unavailable = 此剪贴板操作已不可用，请重新打开剪贴板历史后重试。
error-open-history = 无法打开剪贴板历史，请查看详细信息后重试。

error-update-history = 无法更新剪贴板历史，当前视图已保留。请重试或重新打开视图。
error-close-history = 无法关闭剪贴板历史，请再次尝试返回。
error-history-changed = 剪贴板历史已变化，请重新打开后继续。
error-history-interaction = 剪贴板历史中无法进行此操作，请重新打开后重试。
error-copied-monitor-failed = 条目已复制，但剪贴板跟踪未能完成。请先检查剪贴板内容，再决定是否重试。

error-apply-configuration = 无法应用剪贴板历史设置。请检查详情后重试。
error-invalid-configuration = 剪贴板历史设置无效，请检查设置并修正报告的值。

error-unsupported-protocol = 剪贴板历史不支持请求的扩展协议。
error-not-initialized = 剪贴板历史必须先完成初始化才能处理请求。
error-unsupported-message = 剪贴板历史收到了不支持的请求。

error-host-service = { $outcome ->
    [cancelled] 复制历史条目的请求已在受理前取消。
    [rejected] 宿主未受理复制历史条目的请求，请查看详情后重试。
   *[failed] 复制历史条目时报告了失败，请查看详情后再决定是否重试。
    [unconfirmed] Nanika 无法确认历史条目是否已复制，请先检查剪贴板内容，再决定是否重试。
    }
