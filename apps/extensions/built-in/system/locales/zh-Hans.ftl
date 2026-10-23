extension-name = 系统

action-lock-title = 锁定屏幕
action-sleep-title = 睡眠
action-displays-title = 关闭显示器
action-logout-title = { $platform ->
    [windows] 注销
   *[macos] 退出登录
    }
action-logout-confirmation-title = { $platform ->
    [windows] 注销？
   *[macos] 退出登录？
    }
action-logout-confirmation-description = 当前会话将结束，已打开的应用将关闭。请先保存你的工作。
action-restart-title = 重新启动
action-restart-confirmation-title = 重新启动？
action-restart-confirmation-description = 电脑将重新启动，已打开的应用将关闭。请先保存你的工作。
action-shutdown-title = 关机
action-shutdown-confirmation-title = 关机？
action-shutdown-confirmation-description = 电脑将关机，已打开的应用将关闭。请先保存你的工作。
action-trash-open-title = { $platform ->
    [windows] 打开回收站
   *[macos] 打开废纸篓
    }
action-trash-empty-title = { $platform ->
    [windows] 清空回收站
   *[macos] 清倒废纸篓
    }
action-trash-empty-confirmation-title = { $platform ->
    [windows] 清空回收站？
   *[macos] 清倒废纸篓？
    }
action-trash-empty-confirmation-description = { $platform ->
    [windows] 回收站中的所有项目将被永久删除。此操作无法撤销。
   *[macos] 废纸篓中的所有项目将被永久删除。此操作无法撤销。
    }

command-lock-description = 锁定当前会话
command-sleep-description = 让电脑进入睡眠状态
command-displays-description = 让已连接的显示器进入睡眠状态
command-logout-description = 结束当前用户会话
command-restart-description = 重新启动这台电脑
command-shutdown-description = 关闭这台电脑
command-trash-open-description = 在系统文件管理器中查看已删除的项目
command-trash-empty-description = { $platform ->
    [windows] 永久删除回收站中的所有项目
   *[macos] 永久删除废纸篓中的所有项目
    }
keyword-delete = 删除
keyword-display = 显示器
keyword-empty-recycle-bin = 清空回收站
keyword-lock = 锁屏
keyword-logout = 退出登录
keyword-monitor = 显示器
keyword-power-off = 关闭电源
keyword-reboot = 重启
keyword-recycle-bin = 回收站
keyword-restart = 重新启动
keyword-screen = 屏幕
keyword-shutdown = 关机
keyword-sign-out = 注销
keyword-sleep = 睡眠
keyword-suspend = 挂起
keyword-trash = 废纸篓
keyword-turn-off-display = 关闭显示器

error-already-initialized = 系统扩展已完成初始化。
error-unsupported-protocol = 系统扩展不支持请求的扩展协议。
error-not-initialized = 系统扩展必须先完成初始化才能处理请求。
error-busy = 有一个系统操作正在等待处理，请等待完成后重试。
error-unknown-action = 此系统操作不可用。
error-invalid-response = 系统服务返回了非预期的响应，请先确认操作是否已完成，再决定是否重试。
error-unsupported-message = 系统扩展收到了不支持的请求。

error-host-service = { $outcome ->
    [cancelled] 系统操作已在受理前取消。
    [rejected] 宿主未受理此系统操作，请查看详情后重试。
   *[failed] 系统服务报告了失败，请查看详情后再决定是否重试。
    [unconfirmed] Nanika 无法确认系统操作是否已提交，请先检查系统状态，再决定是否重试。
    }
