-brand-name = Nanika
-greeting = 你好，{ $person }！
brand = 欢迎使用 { -brand-name }。
salutation = { -greeting(person: "Ada") }
integer = 数量：{ $count }
files = { $count ->
    [0] 没有文件
   *[other] { $count } 个文件
    }
platform = { $platform ->
    [macos] 清倒废纸篓
   *[windows] 清空回收站
    }
content = 内容：{ $value }
reference = { integer }
files-more = { $name } 及其他 { $count } 项
