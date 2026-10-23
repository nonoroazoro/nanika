extension-name = 应用程序
setting-application-builtin-macos-system-applications-title = 系统应用
setting-application-builtin-macos-system-applications-description = 搜索系统应用文件夹。
group-search-locations = 搜索位置
setting-application-builtin-macos-user-applications-title = 用户应用
setting-application-builtin-macos-user-applications-description = 搜索用户文件夹中的应用。
setting-application-builtin-windows-packaged-title = 打包应用
setting-application-builtin-windows-packaged-description = 搜索已注册且当前用户可用的打包应用。
setting-application-builtin-windows-system-programs-title = 系统开始菜单
setting-application-builtin-windows-system-programs-description = 搜索 Windows 共享开始菜单。
setting-application-builtin-windows-scoop-global-title = 全局 Scoop 快捷命令
setting-application-builtin-windows-scoop-global-description = 搜索系统级 Scoop 安装中可用的快捷命令。
setting-application-builtin-windows-scoop-user-title = 用户 Scoop 快捷命令
setting-application-builtin-windows-scoop-user-description = 搜索当前用户 Scoop 安装中可用的快捷命令。
setting-application-builtin-windows-user-programs-title = 用户开始菜单
setting-application-builtin-windows-user-programs-description = 搜索当前用户的 Windows 开始菜单。
setting-application-roots-title = 文件夹
setting-application-roots-description = 除系统应用外，也搜索这些文件夹。

action-open = 打开
action-reveal-shortcut = 打开快捷方式所在位置
action-reveal-finder = 在访达中显示
action-reveal-file = 打开文件所在位置
action-reveal-target = 打开目标所在位置
group-location = 位置

candidate-kind = 应用程序

progress-finding-sources = 正在查找应用来源
progress-scanning-sources = 正在扫描应用来源
progress-updating-index = 正在更新应用索引

error-apply-configuration = 无法完成应用设置的更新。请检查详情后重试。
error-invalid-configuration = 应用设置无效。请检查设置并修正报告的值。
error-refresh-catalog = 无法刷新应用目录，请查看详情后重试。

error-read-catalog = 无法读取应用目录，请查看详情后重试。
error-action-unavailable = 此应用操作已不可用，请重新搜索该应用。
error-invalid-host-response = 宿主返回了非预期的应用操作响应，请先确认操作是否已完成，再决定是否重试。
error-refresh-cancelled = 应用扫描已取消。
error-unsupported-protocol = 应用扩展不支持请求的扩展协议。
error-not-initialized = 应用扩展必须先完成初始化才能处理请求。
error-unsupported-message = 应用扩展收到了不支持的请求。

error-host-service = { $outcome ->
    [cancelled] 应用操作已在受理前取消。
    [rejected] 宿主未受理此应用操作，请查看详情后重试。
   *[failed] 应用操作报告了失败，请查看详情后再决定是否重试。
    [unconfirmed] Nanika 无法确认应用操作的结果，请先检查应用或文件管理器，再决定是否重试。
    }
