settings-general = 通用
settings-launcher = 启动器
settings-open-launcher = 打开启动器
settings-hide-on-blur = 失去焦点时隐藏
settings-appearance = 外观
settings-theme = 主题
settings-theme-system = 跟随系统
settings-theme-light = 浅色
settings-theme-dark = 深色
settings-language = 语言
settings-language-system = 跟随系统
settings-startup = 启动
settings-launch-at-login = 登录时启动
settings-launch-at-login-description = 登录后自动启动 Nanika。
settings-startup-approval = 开启此选项以打开系统设置并允许 Nanika 启动。
settings-startup-repair = 开启此选项以修复登录时启动功能。
settings-about = 关于
tray-open = 打开 Nanika
tray-quit = 退出
settings-window-title = Nanika 设置
native-add-folder = 添加文件夹
settings-enable-extension = 启用{ $name }
settings-enable-extension-title = 启用扩展
extension-menu-label = { $name }菜单
localization-load-failed = 无法加载翻译。
action-retry = 重试

settings-progress-units = { $label }：{ $completed } / { $total }
settings-progress-updating = 正在更新{ $label }

error-configuration-save-failed = 更改无法保存，尚未应用。
error-configuration-applied-not-saved = 更改已生效，但无法保存。
error-configuration-unavailable = 扩展不可用，更改未保存。
error-configuration-unconfirmed-not-saved = 无法确认更改是否生效，更改未保存。
error-configuration-saved-not-applied = 更改已保存，但无法应用。
notification-dismiss = 关闭通知

validation-unknown-setting = 未知设置。
validation-whole-number = 请输入整数。
validation-integer-minimum = 请输入不小于 { $value } 的整数。
validation-integer-maximum = 请输入不大于 { $value } 的整数。
validation-integer-multiple = 请输入 { $value } 的整数倍。
validation-integer-limit = 请选择有限值或无限制。
validation-text-required = 请输入文本。
validation-text-bytes = 请输入不超过 { $value } 个 UTF-8 字节的文本。
validation-boolean-required = 请选择开启或关闭。
validation-array-required = 请输入列表。
validation-array-length = 最多可输入 { $value } 个条目。
validation-object-required = 请输入对象。
validation-entry = 第 { $index } 个条目
validation-field = { $field }：{ $error }
settings-limit-finite-label = { $label }：使用上限 { $value }
settings-limit-finite-title = 使用数值上限
settings-limit-unlimited-label = { $label }：无限制
settings-limit-unlimited-title = 无限制
settings-array-entry-label = { $label }，第 { $index } 个条目
settings-array-remove-label = 移除{ $label }的第 { $index } 个条目
settings-array-remove = 移除
settings-array-add = 添加条目
settings-array-show-more = 显示更多（{ $count }）
settings-array-count = { $count } / { $maximum }
settings-object-include = 包含
settings-object-include-label = 包含{ $field }
settings-object-field-label = { $label }，{ $field }

settings-title = 设置
settings-navigation = 设置导航
settings-content = 设置内容
settings-loading = 正在加载设置…
settings-search = 搜索设置
settings-search-placeholder = 搜索
settings-search-clear = 清除搜索
settings-search-results = 设置搜索结果
settings-sections = 设置分类
settings-search-empty = 未找到设置
settings-search-count = { $count } 个结果
settings-extensions = 扩展
settings-about-description = 应用、命令和扩展的统一入口。
settings-app-information = 应用信息
settings-version = 版本
window-controls = 窗口控制
window-minimize = 最小化
window-maximize = 最大化
window-restore = 还原
window-close-settings = 关闭设置
settings-folder-add = 添加文件夹
settings-folder-remove = 移除文件夹
settings-shortcut-label = 打开启动器的快捷键
settings-shortcut-recording = 请按快捷键…
settings-shortcut-modifiers = { $platform ->
    [macos] 请同时按下 Control、Option 或 Command 与另一个键。
   *[windows] 请同时按下 Ctrl、Alt 或 Windows 键与另一个键。
    }
extension-state = { $state ->
    [disabled] 已停用
    [dormant] 按需启动
    [starting] 正在启动…
    [ready] 运行中
    [stopping] 正在停止…
   *[failed] 不可用
    }
launcher-label = Nanika 启动器
launcher-search = 搜索应用和命令
launcher-results = 搜索结果
launcher-searching = 正在搜索扩展…
launcher-searching-count = 正在搜索 { $count } 个扩展…
launcher-empty = 没有搜索结果
search-empty-description = 请尝试其他搜索词。
launcher-menu = Nanika 菜单
launcher-unavailable = 搜索暂不可用。
launcher-display-failed = 无法显示搜索结果。
actions-label = 操作
action-cancel = 取消
action-confirm = 确认
view-label = 扩展视图
view-back = 返回上一视图
view-search = 搜索当前视图
view-content-filter = 内容筛选
view-details = 详情
view-items = 项目
view-loading-items = 正在加载项目…
view-items-load-failed = 无法加载项目。
view-text-content = 详情内容
view-text-load-more = 加载更多文本
view-files = 文件
view-file-collection = 文件集合
view-file-collection-label = 包含 { $count } 个项目的集合
view-file-count = { $count } 个项目
view-file-paths = { $count ->
    [one] 路径
   *[other] 路径
    }
view-files-more = 还有 { $count } 个文件

error-operation-extension-unavailable = 扩展已不可用，请重新打开视图后重试。
error-operation-cancelled = 请求已取消。
error-operation-extension-terminated = 扩展在回复前已停止，系统已接受的操作仍可能完成。
error-operation-extension-io = 与扩展通信失败，请重新打开视图后重试。
error-operation-extension-protocol = 扩展返回了无效响应，请重新打开视图后重试。
error-operation-text-read-failed = 无法加载更多文本，请重试。
error-operation-text-read-timeout = 加载更多文本超时，请重试。
error-operation-text-source-changed = 文本来源已更改，请等待视图刷新。
error-operation-invalid-text-read = 文本请求无效，请重新打开视图后重试。
error-operation-session-unavailable = 当前窗口会话已不可用，请重新打开 Nanika。
error-operation-runtime-unavailable = Nanika 尚无法处理此请求，请稍后重试。

error-operation-search-changed = 搜索结果已变化，请选择当前结果后重试。
error-operation-action-unavailable = 此操作已不可用，请重新打开菜单或视图后重试。
error-operation-operation-pending = 另一项操作仍在进行，请等待完成后重试。
error-operation-invocation-failed = 无法完成操作，请查看详细信息后再决定是否重试。
error-operation-navigation-failed = 无法打开或关闭视图。已接受的操作可能已经完成，请重新打开 Nanika 后继续。
error-operation-view-operation-failed = 无法更新视图，请查看详细信息，必要时重新打开视图。

error-operation-invocation-unconfirmed = 无法确认操作结果，操作可能已经完成。请先检查结果，再决定是否重试。
error-operation-settings-navigation-unsupported = 此扩展尝试从设置中打开启动器视图，请在启动器中重新打开此操作。

error-operation-view-operation-unconfirmed = 无法确认视图操作的结果。已接受的操作可能已经完成，请重新打开视图并检查结果，再决定是否重试。
error-operation-view-changed = 视图已变化，请使用当前视图或重新打开。
error-operation-view-interaction-unavailable = 当前视图不支持此操作，请重新打开视图后重试。
error-operation-confirmation-changed = 确认后视图发生了变化，请检查当前操作并重新确认。
error-operation-navigation-depth-reached = 打开的视图过多，请先返回上一层，再打开其他视图。
error-operation-cleanup-failed = 释放新视图也失败了：

error-operation-configuration-unconfirmed = 没有收到配置操作的完成结果。

error-operation-refresh-unconfirmed = 扩展已关闭，未确认目录刷新是否完成。
error-operation-host-services-unavailable = 某项系统服务不可用，部分扩展功能可能无法使用。
error-operation-extension-storage-unavailable = 无法加载扩展记录，请查看详情后重试。
error-operation-extension-package-unavailable = 无法加载扩展包，请查看详情后重试。
error-operation-extension-icon-unavailable = 无法加载扩展图标。
error-operation-extension-configuration-unavailable = 无法加载扩展设置，请查看详情后重试。

error-operation-extension-exited = 扩展意外停止。
error-operation-extension-stop-unconfirmed = 扩展工作线程已退出，未确认停止是否完成。
error-operation-extension-still-stopping = 上一个扩展实例仍在停止中，请等待其停止后再启用扩展。
error-operation-extension-registry-save-failed = 无法保存扩展的启用设置，原设置保持不变。
error-operation-extension-lifecycle-unconfirmed = 扩展操作已结束，但结果尚未确认。请检查当前状态后再重试。
error-operation-extension-lifecycle-failed = 无法更改扩展的启用状态，请查看详情后重试。
error-operation-catalog-retirement-failed = 无法撤回扩展的搜索结果。
error-operation-application-shutting-down = Nanika 正在退出。
error-lifecycle-restarting = 扩展运行失败，正在进行一次自动重启。
error-lifecycle-stopping = 重启后的扩展再次运行失败，正在停止其实例。
error-lifecycle-restart-failed = 无法启动自动重启的实例，请禁用后重新启用扩展以重试。
error-lifecycle-restart-limit = 扩展在一次自动重启后再次运行失败，已停止自动重启。请禁用后重新启用扩展以重试。
error-lifecycle-recovery-stopped = 扩展恢复已停止，请查看以下失败原因。
error-lifecycle-original = 首次失败
error-lifecycle-restarted = 重启失败
error-lifecycle-stop = 恢复失败

error-configuration-rejected = 此更改未被接受，没有保存或应用。
error-configuration-unconfirmed = 无法确认更改是否已保存或生效。请先检查当前设置，再决定是否重试。
error-operation-configuration-changed = 设置已更改，请重新打开菜单后重试。
error-operation-configuration-property-unavailable = 此设置在这里不可用。
error-operation-invalid-configuration = 此设置不符合要求，请检查输入后重试。
error-operation-configuration-submission-failed = 设置操作未能启动，请重试。

error-operation-runtime-startup-failed = Nanika 无法启动。
error-operation-built-in-inventory-unavailable = Nanika 无法加载内置扩展。
error-operation-host-configuration-unavailable = Nanika 无法打开配置。
error-operation-extension-registry-unavailable = Nanika 无法读取扩展注册表。
error-operation-host-storage-unavailable = Nanika 无法打开或更新数据库。
error-operation-search-unavailable = Nanika 无法启动搜索。
error-operation-startup-diagnostic-unavailable = 启动诊断不可用。
launcher-startup-recovery = 查看诊断并解决所报告的问题，然后重新启动 Nanika。
launcher-show-diagnostics = 查看诊断
launcher-hide-diagnostics = 隐藏诊断
launcher-loading-diagnostics = 正在加载诊断…
launcher-diagnostics = 启动诊断
launcher-no-diagnostics = 没有记录其他诊断信息。
error-operation-invalid-search-request = 搜索请求无效，请修改输入后重试。
error-operation-search-query-too-long = 搜索内容过长，请缩短后重试。
error-operation-search-submission-failed = 无法更新搜索，请修改输入后重试。
error-operation-settings-search-failed = 无法搜索设置，请重试。
error-operation-settings-read-failed = 无法读取设置，请重试。
error-operation-settings-open-failed = 无法打开设置窗口，请重试。
error-operation-settings-ready-failed = 设置窗口未能完成更新，请重试。
error-operation-window-drag-failed = 无法移动窗口，请重新拖动。
error-operation-window-minimize-failed = 无法最小化窗口，请重试。
error-operation-window-resize-failed = 无法调整窗口大小，请重试。
error-operation-window-close-failed = 无法隐藏窗口，请重试。
error-operation-startup-read-failed = 无法读取登录时启动状态，请重试。
error-operation-startup-write-unconfirmed = 无法确认登录时启动设置是否已更改，请检查当前状态后重试。
search-input-too-long = 搜索最多支持 { $maximum } 个字符，请修改输入后继续。
error-operation-invalid-host-preferences = 启动器偏好设置无效，请检查后重试。
error-operation-invalid-shortcut = 快捷键无效，请重新录制。
error-operation-shortcut-registration-failed = 无法注册快捷键，请检查是否被其他应用占用。
error-operation-shortcut-release-failed = 无法释放原快捷键，请检查当前快捷键后重试。
error-operation-shortcut-cleanup-failed = 恢复时无法释放新快捷键，它可能仍然生效。
error-operation-preferences-save-failed = 无法保存启动器偏好设置，请检查当前设置后重试。
error-operation-preferences-rollback-failed = 无法恢复原偏好设置，已保存的设置可能与当前生效的设置不同。
error-operation-preferences-confirmation-failed = 无法确认已保存的偏好设置，请检查当前设置后重试。
error-operation-preferences-unconfirmed = 无法确认偏好设置是否已更改，请检查当前设置后重试。
error-operation-native-language-update-failed = 偏好设置已保存，但原生界面语言更新失败，请重试语言设置。
error-operation-language-update-failed = 无法更新语言，请重试。
error-operation-directory-setting-unavailable = 此设置已不接受文件夹，请重新打开设置后重试。
error-operation-directory-picker-failed = 无法完成文件夹选择，请重试。
error-operation-directory-unavailable = 所选文件夹不可用，请选择其他文件夹。
error-operation-directory-path-invalid = 不支持此文件夹路径，请选择其他文件夹。
error-operation-shortcut-recording-failed = 无法更新快捷键录制状态，请重试。
error-operation-session-open-failed = 无法打开启动器会话，请重新打开启动器后重试。
error-operation-search-acknowledgement-failed = 无法确认搜索更新，请重新打开启动器后重试。
error-operation-invalid-result-range = 请求的结果范围无效，请重新打开启动器后重试。
error-operation-localization-busy = 正在加载的翻译过多，请等待完成后重试。
error-operation-localization-scope-invalid = 翻译请求无效，请重新打开扩展后重试。
error-operation-localization-resources-unavailable = 扩展没有可用的翻译，请检查扩展包后重试。
error-operation-localization-package-changed = 翻译属于其他扩展包，请重新打开扩展后重试。
error-operation-localization-instance-retired = 这些翻译所属的扩展实例已结束，请重新打开扩展后重试。
