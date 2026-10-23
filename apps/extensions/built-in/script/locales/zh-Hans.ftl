extension-name = 脚本
setting-script-roots-title = 文件夹
setting-script-roots-description = 在这些文件夹中查找 PowerShell、Python、JavaScript 和 Shell 脚本。
group-search-locations = 搜索位置

action-run = 运行

error-apply-configuration = 无法应用脚本设置。请检查详情后重试。
error-invalid-configuration = 脚本设置无效。请检查设置并修正报告的值。

error-read-catalog = 无法读取脚本目录，请查看详情后重试。
error-script-unavailable = 此脚本不可用，请查看详情并检查脚本文件后重试。
error-invalid-host-response = 宿主返回了非预期的脚本操作响应，请先确认操作是否已完成，再决定是否重试。
error-discovery = 无法发现脚本，请查看详情并检查配置的文件夹后重试。
error-unsupported-protocol = 脚本扩展不支持请求的扩展协议。
error-not-initialized = 脚本扩展必须先完成初始化才能处理请求。
error-unsupported-message = 脚本扩展收到了不支持的请求。

error-host-service = { $outcome ->
    [cancelled] 脚本操作已在受理前取消。
    [rejected] 宿主未受理此脚本操作，请查看详情后重试。
   *[failed] 脚本操作报告了失败，请查看详情后再决定是否重试。
    [unconfirmed] Nanika 无法确认脚本是否已启动，请先检查执行情况，避免重复运行。
    }
